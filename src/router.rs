use std::collections::HashMap;
use axum::{
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use axum_login::AuthSession;
use tera::Context;
use serde::{Deserialize, Serialize};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use gray_matter::{engine::YAML, Matter, ParsedEntity};
use pulldown_cmark::{html, Parser};
use tower_http::services::ServeDir;
use walkdir::WalkDir;

use crate::AppState;
use crate::auth::{Backend, Credentials};

// --- Data Structures ---
#[derive(Debug, Deserialize, Serialize)] struct ProblemMeta { title: String, time_limit: String, memory_limit: String, tags: Vec<String> }
#[derive(Debug, Serialize)] struct ProblemListItem { id: u32, title: String, accuracy: String }
#[derive(Debug, Serialize)] struct ProblemDetailView { id: u32, meta: ProblemMeta, content: String, total_submits: i64, correct_submits: i64, accuracy: String }
#[derive(Deserialize)] pub struct AuthQuery { error: Option<String> }
pub struct AppError(anyhow::Error);

// --- Router Definition ---
pub fn create_router() -> Router<AppState> {
    Router::new()
        .route("/", get(root))
        .route("/learn", get(learn_page))
        .route("/problems", get(problems_list))
        .route("/problems/:id", get(problem_detail))
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
async fn problems_list( State(state): State<AppState>, auth_session: AuthSession<Backend>) -> Result<Html<String>, AppError> {
    let matter = Matter::<YAML>::new();
    let mut problems = Vec::new();
    for entry in WalkDir::new("problems").into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            let path = entry.path();
            if let Some(id_str) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(id) = id_str.parse::<u32>() {
                    let content = tokio::fs::read_to_string(path).await?;
                    let parsed: ParsedEntity = matter.parse(&content);
                    let meta: ProblemMeta = parsed.data.ok_or_else(|| anyhow::anyhow!("Front matter missing"))?.deserialize()?;
                    problems.push(ProblemListItem { id, title: meta.title, accuracy: "0.000%".to_string() });
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

#[axum::debug_handler]
async fn problem_detail( State(state): State<AppState>, auth_session: AuthSession<Backend>, Path(problem_id): Path<u32>) -> Result<Html<String>, AppError> {
    let folder_num = if problem_id == 0 { 0 } else { ((problem_id - 1) / 1000 + 1) * 1000 };
    let file_path = format!("problems/{:06}/{}.md", folder_num, problem_id);
    let content = tokio::fs::read_to_string(&file_path).await?;
    let matter = Matter::<YAML>::new();
    let parsed_entity = matter.parse(&content);
    let meta: ProblemMeta = parsed_entity.data.ok_or_else(|| anyhow::anyhow!("Front matter missing"))?.deserialize()?;
    let parser = Parser::new(&parsed_entity.content);
    let mut html_content = String::new();
    html::push_html(&mut html_content, parser);
    let view_data = ProblemDetailView { id: problem_id, meta, content: html_content, total_submits: 0, correct_submits: 0, accuracy: "0.000%".to_string() };
    let mut context = Context::new();
    context.insert("problem", &view_data);
    context.insert("active_page", "problems");
    context.insert("current_user", &auth_session.user);
    let rendered = state.tera.render("problem.html", &context)?;
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

