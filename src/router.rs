use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use axum::routing::post;
use axum_login::AuthSession;
use gray_matter::{engine::YAML, Matter, ParsedEntity};
use pulldown_cmark::{html, Parser};
use serde::{Deserialize, Serialize};
use tera::Context;
use tower_http::services::ServeDir;
use sqlx::Row;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::auth::{Backend, Credentials};
use crate::AppState;

// --- Data Structures ---
#[derive(Debug, Deserialize, Serialize)] struct ProblemMeta { title: String, time_limit: String, memory_limit: String, tags: Vec<String> }
#[derive(Debug, Serialize)] struct ProblemListItem { id: u32, title: String, accuracy: String }
#[derive(Debug, Serialize)] struct ProblemDetailView {
    id: u32,
    meta: ProblemMeta,
    content: String,
    total_submits: i64,
    correct_submits: i64,
    accuracy: String,
    example_inputs: Vec<String>,
    example_outputs: Vec<String>,
}
#[derive(Deserialize)] pub struct AuthQuery { error: Option<String> }
pub struct AppError(anyhow::Error);

// --- Router Definition ---
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/learn", get(learn_page))
        .route("/problems", get(problems_list))
        .route("/problems/:id", get(problem_detail))
        .route("/problems/:id/submit", post(submit_solution))
        .route("/problems/:id/status", get(problem_status))
        .route("/submissions/:id", get(submission_detail)) // 새로 추가
        .route("/login", get(login_page).post(login_action))
        .route("/register", get(register_page).post(register_action))
        .route("/logout", get(logout_action))
        .nest_service("/static", ServeDir::new("static"))
}

// --- Page Handlers ---

#[axum::debug_handler]
async fn root( State(state): State<AppState>, auth_session: AuthSession<Backend>) -> Result<Html<String>, AppError> {
    let mut context = Context::new();
    context.insert("current_user", &auth_session.user);
    context.insert("active_page", "home");
    let rendered = state.tera.render("index.html", &context)?;
    Ok(Html(rendered))
}

#[axum::debug_handler]
async fn learn_page( State(state): State<AppState>, auth_session: AuthSession<Backend>) -> Result<Html<String>, AppError> {
    let mut context = Context::new();
    context.insert("current_user", &auth_session.user);
    context.insert("active_page", "learn");
    let rendered = state.tera.render("learn.html", &context)?;
    Ok(Html(rendered))
}

#[axum::debug_handler]
async fn problems_list(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>
) -> Result<Html<String>, AppError> {
    let matter = Matter::<YAML>::new();
    let mut problems = Vec::new();
    let mut thousand_entries = tokio::fs::read_dir("problems").await?;

    while let Some(thousand_folder) = thousand_entries.next_entry().await? {
        if thousand_folder.file_type().await?.is_dir() {
            let thousand_path = thousand_folder.path();
            let mut problem_entries = tokio::fs::read_dir(&thousand_path).await?;

            while let Some(problem_folder) = problem_entries.next_entry().await? {
                if problem_folder.file_type().await?.is_dir() {
                    if let Some(id_str) = problem_folder.file_name().to_str() {
                        if let Ok(id) = id_str.parse::<u32>() {
                            let md_file = problem_folder.path().join(format!("{}.md", id));

                            if tokio::fs::metadata(&md_file).await.is_ok() {
                                let content = tokio::fs::read_to_string(&md_file).await?;
                                let parsed: ParsedEntity = matter.parse(&content);

                                if let Some(data) = parsed.data {
                                    if let Ok(meta) = data.deserialize::<ProblemMeta>() {
                                        // 정확률 계산
                                        let accuracy = calculate_accuracy(&state, id).await?;

                                        problems.push(ProblemListItem {
                                            id,
                                            title: meta.title,
                                            accuracy,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    problems.sort_by_key(|p| p.id);
    let mut context = Context::new();
    context.insert("problems", &problems);
    context.insert("active_page", "problems");
    context.insert("current_user", &auth_session.user);
    let rendered = state.tera.render("problems_list.html", &context)?;
    Ok(Html(rendered))
}

async fn calculate_accuracy(state: &AppState, problem_id: u32) -> Result<String, sqlx::Error> {
    let stats = sqlx::query(
        r#"
        SELECT
            COUNT(*) as total,
            COUNT(CASE WHEN status = 'ACCEPTED' THEN 1 END) as accepted
        FROM submissions
        WHERE problem_id = ?
        "#
    )
        .bind(problem_id)
        .fetch_one(&state.db_pool)
        .await?;

    let total: i64 = stats.get("total");
    let accepted: i64 = stats.get("accepted");

    if total == 0 {
        Ok("0.0%".to_string())
    } else {
        let accuracy = (accepted as f64 / total as f64) * 100.0;
        Ok(format!("{:.1}%", accuracy))
    }
}

#[axum::debug_handler]
async fn problem_detail(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(problem_id): Path<u32>,
) -> Result<Html<String>, AppError> {
    let folder_num = if problem_id < 1000 { 1000 } else { (problem_id / 1000) * 1000 };
    let current_dir = std::env::current_dir()?;
    let file_path = current_dir
        .join("problems")
        .join(format!("{:06}", folder_num))
        .join(problem_id.to_string())
        .join(format!("{}.md", problem_id));

    let content = tokio::fs::read_to_string(&file_path).await
        .map_err(|e| anyhow::anyhow!("파일을 찾을 수 없습니다: {} (경로: {:?})", e, file_path))?;

    let matter = Matter::<YAML>::new();
    let parsed_entity = matter.parse(&content);
    let meta: ProblemMeta = parsed_entity.data
        .ok_or_else(|| anyhow::anyhow!("Front matter missing"))?
        .deserialize()?;

    // DB에서 제출 통계 가져오기
    let stats = sqlx::query(
        r#"
        SELECT
            COUNT(*) as total_submits,
            COUNT(CASE WHEN status = 'ACCEPTED' THEN 1 END) as correct_submits
        FROM submissions
        WHERE problem_id = ?
        "#
    )
        .bind(problem_id)
        .fetch_one(&state.db_pool)
        .await?;

    let total_submits: i64 = stats.get("total_submits");
    let correct_submits: i64 = stats.get("correct_submits");

    let accuracy = if total_submits == 0 {
        "0.0%".to_string()
    } else {
        format!("{:.1}%", (correct_submits as f64 / total_submits as f64) * 100.0)
    };

    let (html_content, example_inputs, example_outputs) = extract_examples(&parsed_entity.content);

    let view_data = ProblemDetailView {
        id: problem_id,
        meta,
        content: html_content,
        total_submits,
        correct_submits,
        accuracy,
        example_inputs,
        example_outputs,
    };

    let mut context = Context::new();
    context.insert("problem", &view_data);
    context.insert("active_page", "problems");
    context.insert("current_user", &auth_session.user);
    let rendered = state.tera.render("problem.html", &context)?;
    Ok(Html(rendered))
}

#[derive(Deserialize)]
struct SubmissionForm {
    language: String,
    source_code: String,
}

#[derive(Debug, Serialize)]
struct SubmissionStatus {
    id: i64,
    username: String,
    language: String,
    status: String,
    score: Option<i32>,
    execution_time: Option<i32>,
    memory_usage: Option<i32>,
    compile_message: Option<String>,
    runtime_error_type: Option<String>,
    runtime_error_message: Option<String>,
    total_testcases: i32,
    passed_testcases: i32,
    submitted_at: String,
    judged_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct TestcaseResult {
    testcase_number: i32,
    status: String,
    execution_time: Option<i32>,
    memory_usage: Option<i32>,
    error_message: Option<String>,
}

#[derive(Debug, Serialize)]
struct SubmissionDetail {
    submission: SubmissionStatus,
    testcase_results: Vec<TestcaseResult>,
}

#[derive(Debug, Serialize)]
struct ProblemStatusView {
    id: u32,
    title: String,
    submissions: Vec<SubmissionStatus>,
}

fn extract_examples(content: &str) -> (String, Vec<String>, Vec<String>) {
    // Lazy 정적 변수를 사용하여 정규식을 한 번만 컴파일합니다.
    static RE_EXAMPLES: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?s)### (예제 입력|예제 출력)\s*(\d+)\s*\n+```\n?(.*?)\n?```").unwrap()
    });

    let mut example_inputs: Vec<(u32, String)> = Vec::new();
    let mut example_outputs: Vec<(u32, String)> = Vec::new();

    // 정규식을 사용해 모든 예제(입력/출력)를 찾습니다.
    for cap in RE_EXAMPLES.captures_iter(content) {
        let kind = &cap[1]; // "예제 입력" 또는 "예제 출력"
        let num = cap[2].parse::<u32>().unwrap_or(0);
        // 정규식이 코드 블록 안의 내용만 정확히 가져오므로 trim()만으로 충분합니다.
        let code = cap[3].trim().to_string();

        if kind == "예제 입력" {
            example_inputs.push((num, code));
        } else if kind == "예제 출력" {
            example_outputs.push((num, code));
        }
    }

    // 예제 번호 순서대로 정렬합니다.
    example_inputs.sort_by_key(|k| k.0);
    example_outputs.sort_by_key(|k| k.0);

    // 정렬된 결과에서 텍스트만 추출합니다.
    let final_inputs: Vec<String> = example_inputs.into_iter().map(|(_, text)| text).collect();
    let final_outputs: Vec<String> = example_outputs.into_iter().map(|(_, text)| text).collect();

    // 원본 콘텐츠에서 예제 부분을 모두 제거합니다.
    let content_without_examples = RE_EXAMPLES.replace_all(content, "").trim().to_string();

    // 예제가 제거된 나머지 콘텐츠만 HTML로 변환합니다.
    let parser = Parser::new(&content_without_examples);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    (html_output, final_inputs, final_outputs)
}

async fn submit_solution(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(problem_id): Path<u32>,
    Form(form): Form<SubmissionForm>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(user) => user,
        None => return Redirect::to("/login").into_response(),
    };

    // DB에 제출 기록 저장 (PENDING 상태로)
    let result = sqlx::query(
        "INSERT INTO submissions (user_id, problem_id, language, source_code, status) VALUES (?, ?, ?, ?, 'PENDING')"
    )
        .bind(user.id)
        .bind(problem_id)
        .bind(&form.language)
        .bind(&form.source_code)
        .execute(&state.db_pool)
        .await;

    match result {
        Ok(result) => {
            let submission_id = result.last_insert_rowid();

            let judge_request = crate::judge::JudgeRequest {
                submission_id,
                language: form.language,
                source_code: form.source_code,
                problem_id,
            };

            let db_pool = state.db_pool.clone();
            tokio::spawn(async move {
                match crate::judge::judge_submission(judge_request).await {
                    Ok(judge_result) => {
                        let _ = sqlx::query(
                            r#"
                            UPDATE submissions SET
                                status = ?, score = ?, execution_time = ?, memory_usage = ?,
                                compile_message = ?, runtime_error_type = ?, runtime_error_message = ?,
                                total_testcases = ?, passed_testcases = ?, judged_at = CURRENT_TIMESTAMP
                            WHERE id = ?
                            "#
                        )
                            .bind(&judge_result.status)
                            .bind(judge_result.score)
                            .bind(judge_result.execution_time)
                            .bind(judge_result.memory_usage)
                            .bind(&judge_result.compile_message)
                            .bind(&judge_result.runtime_error_type)
                            .bind(&judge_result.runtime_error_message)
                            .bind(judge_result.total_testcases)
                            .bind(judge_result.passed_testcases)
                            .bind(submission_id)
                            .execute(&db_pool)
                            .await;

                        for testcase in judge_result.testcase_results {
                            let _ = sqlx::query(
                                r#"
                                INSERT INTO testcase_results
                                    (submission_id, testcase_number, status, execution_time, memory_usage, error_message)
                                VALUES (?, ?, ?, ?, ?, ?)
                                "#
                            )
                                .bind(submission_id)
                                .bind(testcase.testcase_number)
                                .bind(&testcase.status)
                                .bind(testcase.execution_time)
                                .bind(testcase.memory_usage)
                                .bind(&testcase.error_message)
                                .execute(&db_pool)
                                .await;
                        }

                        if let Some(compile_errors) = judge_result.compile_errors {
                            for error in compile_errors {
                                let _ = sqlx::query(
                                    r#"
                                    INSERT INTO compile_errors
                                        (submission_id, line_number, column_number, error_type, error_message)
                                    VALUES (?, ?, ?, ?, ?)
                                    "#
                                )
                                    .bind(submission_id)
                                    .bind(error.line_number)
                                    .bind(error.column_number)
                                    .bind(&error.error_type)
                                    .bind(&error.error_message)
                                    .execute(&db_pool)
                                    .await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = sqlx::query(
                            "UPDATE submissions SET status = 'SYSTEM_ERROR', judged_at = CURRENT_TIMESTAMP WHERE id = ?"
                        )
                            .bind(submission_id)
                            .execute(&db_pool)
                            .await;

                        eprintln!("채점 오류: {}", e);
                    }
                }
            });

            Redirect::to(&format!("/problems/{}", problem_id)).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "제출 실패").into_response(),
    }
}

#[axum::debug_handler]
async fn submission_detail(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(submission_id): Path<i64>,
) -> Result<Html<String>, AppError> {
    // 동적 쿼리 사용
    let submission_row = sqlx::query(
        r#"
        SELECT
            s.id,
            u.username,
            s.language,
            s.status,
            s.score,
            s.execution_time,
            s.memory_usage,
            s.compile_message,
            s.runtime_error_type,
            s.runtime_error_message,
            s.total_testcases,
            s.passed_testcases,
            datetime(s.created_at, 'localtime') as submitted_at,
            datetime(s.judged_at, 'localtime') as judged_at
        FROM submissions s
        JOIN users u ON s.user_id = u.id
        WHERE s.id = ?
        "#
    )
        .bind(submission_id)
        .fetch_one(&state.db_pool)
        .await?;

    let submission = SubmissionStatus {
        id: submission_row.get("id"),
        username: submission_row.get("username"),
        language: submission_row.get("language"),
        status: submission_row.get("status"),
        score: submission_row.get("score"),
        execution_time: submission_row.get("execution_time"),
        memory_usage: submission_row.get("memory_usage"),
        compile_message: submission_row.get("compile_message"),
        runtime_error_type: submission_row.get("runtime_error_type"),
        runtime_error_message: submission_row.get("runtime_error_message"),
        total_testcases: submission_row.get("total_testcases"),
        passed_testcases: submission_row.get("passed_testcases"),
        submitted_at: submission_row.get("submitted_at"),
        judged_at: submission_row.get("judged_at"),
    };

    let testcase_rows = sqlx::query(
        r#"
        SELECT
            testcase_number,
            status,
            execution_time,
            memory_usage,
            error_message
        FROM testcase_results
        WHERE submission_id = ?
        ORDER BY testcase_number
        "#
    )
        .bind(submission_id)
        .fetch_all(&state.db_pool)
        .await?;

    let testcase_results: Vec<TestcaseResult> = testcase_rows
        .into_iter()
        .map(|row| TestcaseResult {
            testcase_number: row.get("testcase_number"),
            status: row.get("status"),
            execution_time: row.get("execution_time"),
            memory_usage: row.get("memory_usage"),
            error_message: row.get("error_message"),
        })
        .collect();

    let submission_detail = SubmissionDetail {
        submission,
        testcase_results,
    };

    let mut context = Context::new();
    context.insert("submission_detail", &submission_detail);
    context.insert("active_page", "problems");
    context.insert("current_user", &auth_session.user);
    let rendered = state.tera.render("submission_detail.html", &context)?;
    Ok(Html(rendered))
}

#[axum::debug_handler]
async fn problem_status(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(problem_id): Path<u32>,
) -> Result<Html<String>, AppError> {
    let folder_num = if problem_id < 1000 { 1000 } else { (problem_id / 1000) * 1000 };
    let current_dir = std::env::current_dir()?;
    let file_path = current_dir
        .join("problems")
        .join(format!("{:06}", folder_num))
        .join(problem_id.to_string())
        .join(format!("{}.md", problem_id));

    let content = tokio::fs::read_to_string(&file_path).await
        .map_err(|e| anyhow::anyhow!("파일을 찾을 수 없습니다: {} (경로: {:?})", e, file_path))?;

    let matter = Matter::<YAML>::new();
    let parsed_entity = matter.parse(&content);
    let meta: ProblemMeta = parsed_entity.data
        .ok_or_else(|| anyhow::anyhow!("Front matter missing"))?
        .deserialize()?;

    let submission_rows = sqlx::query(
        r#"
        SELECT
            s.id,
            u.username,
            s.language,
            s.status,
            s.score,
            s.execution_time,
            s.memory_usage,
            s.compile_message,
            s.runtime_error_type,
            s.runtime_error_message,
            s.total_testcases,
            s.passed_testcases,
            datetime(s.created_at, 'localtime') as submitted_at,
            datetime(s.judged_at, 'localtime') as judged_at
        FROM submissions s
        JOIN users u ON s.user_id = u.id
        WHERE s.problem_id = ?
        ORDER BY s.created_at DESC
        LIMIT 50
        "#
    )
        .bind(problem_id)
        .fetch_all(&state.db_pool)
        .await?;

    let submissions: Vec<SubmissionStatus> = submission_rows
        .into_iter()
        .map(|row| SubmissionStatus {
            id: row.get("id"),
            username: row.get("username"),
            language: row.get("language"),
            status: row.get("status"),
            score: row.get("score"),
            execution_time: row.get("execution_time"),
            memory_usage: row.get("memory_usage"),
            compile_message: row.get("compile_message"),
            runtime_error_type: row.get("runtime_error_type"),
            runtime_error_message: row.get("runtime_error_message"),
            total_testcases: row.get("total_testcases"),
            passed_testcases: row.get("passed_testcases"),
            submitted_at: row.get("submitted_at"),
            judged_at: row.get("judged_at"),
        })
        .collect();

    let view_data = ProblemStatusView {
        id: problem_id,
        title: meta.title,
        submissions,
    };

    let mut context = Context::new();
    context.insert("problem_status", &view_data);
    context.insert("active_page", "problems");
    context.insert("current_user", &auth_session.user);
    let rendered = state.tera.render("problem_status.html", &context)?;
    Ok(Html(rendered))
}


// --- Authentication Handlers (FIXED) ---

#[axum::debug_handler]
async fn login_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>, // auth_session 추가
    Query(query): Query<AuthQuery>,
) -> Result<Html<String>, AppError> {
    let mut context = Context::new();
    // --- FIX: 템플릿 렌더링에 필요한 변수들을 추가했습니다 ---
    context.insert("active_page", ""); // 사이드바 메뉴 활성화를 위함 (빈 값)
    context.insert("current_user", &auth_session.user); // 로그인 상태 표시를 위함

    if let Some(error_key) = query.error {
        context.insert("error", match error_key.as_str() {
            "invalid" => "사용자 이름 또는 비밀번호가 올바르지 않습니다.",
            _ => "알 수 없는 오류가 발생했습니다.",
        });
    }
    let rendered = state.tera.render("login.html", &context)?;
    Ok(Html(rendered))
}

#[axum::debug_handler]
async fn login_action( mut auth_session: AuthSession<Backend>, Form(creds): Form<Credentials>) -> impl IntoResponse {
    let user = match auth_session.authenticate(creds).await { Ok(user) => user, Err(_) => return Redirect::to("/login?error=internal"), };
    if let Some(user) = user {
        if auth_session.login(&user).await.is_ok() { Redirect::to("/") } else { Redirect::to("/login?error=internal") }
    } else { Redirect::to("/login?error=invalid") }
}

#[axum::debug_handler]
async fn register_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>, // auth_session 추가
    Query(query): Query<AuthQuery>,
) -> Result<Html<String>, AppError> {
    let mut context = Context::new();
    // --- FIX: 템플릿 렌더링에 필요한 변수들을 추가했습니다 ---
    context.insert("active_page", ""); // 사이드바 메뉴 활성화를 위함 (빈 값)
    context.insert("current_user", &auth_session.user); // 로그인 상태 표시를 위함

    if let Some(error_key) = query.error {
        context.insert("error", match error_key.as_str() {
            "conflict" => "이미 사용 중인 사용자 이름입니다.",
            "weak_password" => "비밀번호는 8자 이상이어야 합니다.",
            _ => "회원가입 중 오류가 발생했습니다.",
        });
    }
    let rendered = state.tera.render("register.html", &context)?;
    Ok(Html(rendered))
}

#[axum::debug_handler]
async fn register_action( State(state): State<AppState>, Form(creds): Form<Credentials>) -> impl IntoResponse {
    if creds.password.len() < 8 { return Redirect::to("/register?error=weak_password").into_response(); }
    let salt = SaltString::generate(&mut OsRng);
    let hashed_password = Argon2::default().hash_password(creds.password.as_bytes(), &salt).unwrap().to_string();
    match sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(&creds.username).bind(&hashed_password).execute(&state.db_pool).await {
        Ok(_) => Redirect::to("/login").into_response(),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => Redirect::to("/register?error=conflict").into_response(),
        Err(_) => Redirect::to("/register?error=internal").into_response(),
    }
}

#[axum::debug_handler]
async fn logout_action(mut auth_session: AuthSession<Backend>) -> impl IntoResponse {
    auth_session.logout().await.ok();
    Redirect::to("/")
}

// --- Error Handling ---
impl IntoResponse for AppError { fn into_response(self) -> Response { (StatusCode::INTERNAL_SERVER_ERROR, format!("An error occurred: {}", self.0)).into_response() } }
impl<E: Into<anyhow::Error>> From<E> for AppError { fn from(err: E) -> Self { Self(err.into()) } }

