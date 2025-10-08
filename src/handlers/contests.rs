use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
    Form,
    http::StatusCode,
};
use axum_login::AuthSession;
use tera::Context;
use chrono::{DateTime, Utc};

use crate::{auth::Backend, AppState, models::*};

// 날짜 파싱 헬퍼 함수
fn parse_datetime(datetime_str: &str) -> Option<DateTime<Utc>> {
    // RFC3339 형식 (타임존 포함)
    if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // HTML datetime-local 형식 (타임존 없음) - 로컬 시간으로 가정
    // 예: "2025-01-01T10:00"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M") {
        // 로컬 시간대를 고려하여 UTC로 변환
        use chrono::{Local, TimeZone};
        let local_dt = Local.from_local_datetime(&dt);
        if let chrono::LocalResult::Single(local) = local_dt {
            return Some(local.with_timezone(&Utc));
        }
        // fallback: 입력된 시간을 UTC로 직접 해석
        return Some(dt.and_utc());
    }

    // 데이터베이스 형식들
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S") {
        return Some(dt.and_utc());
    }

    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc());
    }

    None
}

// 대회 목록 페이지
pub async fn contests_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "contests");

    let user_id = auth_session.user.as_ref().map(|u| u.id).unwrap_or(0);

    if let Some(user) = &auth_session.user {
        context.insert("current_user", user);
    }

    // 대회 목록 조회 (승인된 대회만 - status로 필터링하지 않고 시간으로 판단)
    let contests_result = sqlx::query_as::<_, Contest>(
        r#"
        SELECT * FROM contests
        WHERE status = 'approved'
        OR (status = 'draft' AND created_by = ?)
        ORDER BY start_time DESC
        "#
    )
    .bind(user_id)
    .fetch_all(&state.db_pool)
    .await;

    match contests_result {
        Ok(contests) => {
            // 현재 시간 기준으로 대회 상태 분류
            let now = Utc::now();
            let mut upcoming = Vec::new();
            let mut active = Vec::new();
            let mut ended = Vec::new();

            for contest in contests {
                // 여러 날짜 형식을 처리
                let start_time = match parse_datetime(&contest.start_time) {
                    Some(dt) => dt,
                    None => {
                        eprintln!("Failed to parse start_time: {}", contest.start_time);
                        continue;
                    }
                };

                let end_time = match parse_datetime(&contest.end_time) {
                    Some(dt) => dt,
                    None => {
                        eprintln!("Failed to parse end_time: {}", contest.end_time);
                        continue;
                    }
                };

                println!("Contest '{}' - Now: {}, Start: {}, End: {}",
                    contest.title, now, start_time, end_time);

                if now < start_time {
                    println!("  -> Upcoming");
                    upcoming.push(contest);
                } else if now >= start_time && now <= end_time {
                    println!("  -> Active");
                    active.push(contest);
                } else {
                    println!("  -> Ended");
                    ended.push(contest);
                }
            }

            context.insert("upcoming_contests", &upcoming);
            context.insert("active_contests", &active);
            context.insert("ended_contests", &ended);
        }
        Err(e) => {
            eprintln!("Failed to fetch contests: {:?}", e);
            context.insert("error", &format!("Failed to load contests: {}", e));
        }
    }

    Html(state.tera.render("contests_list.html", &context).unwrap())
}

// 대회 상세 페이지
pub async fn contest_detail(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "contests");

    if let Some(user) = &auth_session.user {
        context.insert("current_user", user);
    }

    // 대회 정보 조회
    let contest_result = sqlx::query_as::<_, ContestDetail>(
        r#"
        SELECT c.*, u.username as creator_username
        FROM contests c
        JOIN users u ON c.created_by = u.id
        WHERE c.id = ?
        "#
    )
    .bind(id)
    .fetch_one(&state.db_pool)
    .await;

    let contest = match contest_result {
        Ok(c) => c,
        Err(_) => {
            return Html(state.tera.render("error.html", &context).unwrap());
        }
    };

    // 대회 문제 목록 조회
    let problems = sqlx::query_as::<_, ContestProblem>(
        r#"
        SELECT cp.id, cp.problem_id, cp.points, cp.problem_order,
               '' as problem_title
        FROM contest_problems cp
        WHERE cp.contest_id = ?
        ORDER BY cp.problem_order
        "#
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    // 참가자 수 조회
    let participant_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM contest_participants WHERE contest_id = ?"
    )
    .bind(id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or((0,));

    // 현재 사용자가 참가했는지 확인
    let mut is_registered = false;
    if let Some(user) = &auth_session.user {
        let check: Result<(i64,), _> = sqlx::query_as(
            "SELECT id FROM contest_participants WHERE contest_id = ? AND user_id = ?"
        )
        .bind(id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await;
        is_registered = check.is_ok();
    }

    // 대회 상태 판단
    let now = Utc::now();
    println!("Current UTC time: {}", now);

    let start_time = match parse_datetime(&contest.start_time) {
        Some(dt) => dt,
        None => {
            eprintln!("Failed to parse start_time: {}", contest.start_time);
            return Html(state.tera.render("error.html", &context).unwrap());
        }
    };
    let end_time = match parse_datetime(&contest.end_time) {
        Some(dt) => dt,
        None => {
            eprintln!("Failed to parse end_time: {}", contest.end_time);
            return Html(state.tera.render("error.html", &context).unwrap());
        }
    };

    println!("Comparing times:");
    println!("  Now:   {}", now);
    println!("  Start: {}", start_time);
    println!("  End:   {}", end_time);
    println!("  now < start_time: {}", now < start_time);
    println!("  now >= start_time && now <= end_time: {}", now >= start_time && now <= end_time);
    println!("  now > end_time: {}", now > end_time);

    let contest_phase = if now < start_time {
        println!("  -> Phase: upcoming");
        "upcoming"
    } else if now >= start_time && now <= end_time {
        println!("  -> Phase: active");
        "active"
    } else {
        println!("  -> Phase: ended");
        "ended"
    };

    context.insert("contest", &contest);
    context.insert("problems", &problems);
    context.insert("participant_count", &participant_count.0);
    context.insert("is_registered", &is_registered);
    context.insert("contest_phase", &contest_phase);

    Html(state.tera.render("contest_detail.html", &context).unwrap())
}

// 대회 생성 페이지
pub async fn create_contest_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "contests");

    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
        Html(state.tera.render("contest_create.html", &context).unwrap())
    } else {
        Html(state.tera.render("error.html", &context).unwrap())
    }
}

// 대회 생성 액션
pub async fn create_contest_action(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Form(form): Form<CreateContestForm>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    // 대회 생성 (초기 상태는 draft 또는 pending)
    let status = if user.role == "admin" {
        "approved" // 관리자는 바로 승인
    } else {
        "pending" // 일반 사용자는 승인 대기
    };

    let result = sqlx::query(
        r#"
        INSERT INTO contests (title, description, start_time, end_time, contest_type,
                             is_public, max_participants, status, requires_approval, created_by)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, 0, ?)
        "#
    )
    .bind(&form.title)
    .bind(&form.description)
    .bind(&form.start_time)
    .bind(&form.end_time)
    .bind(&form.contest_type)
    .bind(form.is_public)
    .bind(form.max_participants)
    .bind(status)
    .bind(user.id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => Redirect::to("/contests").into_response(),
        Err(e) => {
            eprintln!("Failed to create contest: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to create contest").into_response()
        }
    }
}

// 대회 참가 신청
pub async fn register_contest(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    // 이미 참가했는지 확인
    let already_registered: Result<(i64,), _> = sqlx::query_as(
        "SELECT id FROM contest_participants WHERE contest_id = ? AND user_id = ?"
    )
    .bind(id)
    .bind(user.id)
    .fetch_one(&state.db_pool)
    .await;

    if already_registered.is_ok() {
        return Redirect::to(&format!("/contests/{}", id)).into_response();
    }

    // 대회 참가자 추가
    let result = sqlx::query(
        r#"
        INSERT INTO contest_participants (contest_id, user_id, total_score, penalty_time)
        VALUES (?, ?, 0, 0)
        "#
    )
    .bind(id)
    .bind(user.id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => Redirect::to(&format!("/contests/{}", id)).into_response(),
        Err(e) => {
            eprintln!("Failed to register for contest: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to register").into_response()
        }
    }
}

// 대회 순위표
pub async fn contest_standings(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "contests");

    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }

    // 대회 정보
    let contest = sqlx::query_as::<_, ContestDetail>(
        r#"
        SELECT c.*, u.username as creator_username
        FROM contests c
        JOIN users u ON c.created_by = u.id
        WHERE c.id = ?
        "#
    )
    .bind(id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap();

    // 대회 문제 목록
    let problems = sqlx::query_as::<_, ContestProblem>(
        r#"
        SELECT cp.id, cp.problem_id, cp.points, cp.problem_order,
               '' as problem_title
        FROM contest_problems cp
        WHERE cp.contest_id = ?
        ORDER BY cp.problem_order
        "#
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    // 기본 순위 정보 조회
    let standings_basic = sqlx::query_as::<_, StandingsEntry>(
        r#"
        SELECT
            ROW_NUMBER() OVER (ORDER BY cp.total_score DESC, cp.penalty_time ASC) as rank,
            cp.user_id,
            u.username,
            cp.total_score as solved,
            cp.penalty_time as penalty,
            cp.total_score
        FROM contest_participants cp
        JOIN users u ON cp.user_id = u.id
        WHERE cp.contest_id = ?
        ORDER BY cp.total_score DESC, cp.penalty_time ASC
        "#
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    // 각 참가자의 문제별 제출 상태 조회
    let mut detailed_standings = Vec::new();

    for entry in standings_basic {
        let mut problem_statuses = Vec::new();

        for problem in &problems {
            // 각 문제에 대한 제출 상태 조회
            let submissions: Vec<(String,)> = sqlx::query_as(
                r#"
                SELECT status
                FROM submissions
                WHERE contest_id = ? AND user_id = ? AND problem_id = ?
                ORDER BY created_at ASC
                "#
            )
            .bind(id)
            .bind(entry.user_id)
            .bind(problem.problem_id)
            .fetch_all(&state.db_pool)
            .await
            .unwrap_or_default();

            let mut solved = false;
            let mut attempts = 0;

            for (status,) in submissions {
                if status == "ACCEPTED" {
                    solved = true;
                    break;
                } else if status != "PENDING" && status != "JUDGING" {
                    attempts += 1;
                }
            }

            problem_statuses.push(ProblemSubmissionStatus {
                problem_id: problem.problem_id,
                solved,
                attempts,
                time_minutes: 0, // 나중에 계산 가능
            });
        }

        detailed_standings.push(DetailedStandingsEntry {
            rank: entry.rank,
            user_id: entry.user_id,
            username: entry.username,
            solved: entry.solved,
            penalty: entry.penalty,
            total_score: entry.total_score,
            problem_statuses,
        });
    }

    context.insert("contest", &contest);
    context.insert("problems", &problems);
    context.insert("standings", &detailed_standings);

    Html(state.tera.render("contest_standings.html", &context).unwrap())
}

// 대회 관리 페이지 (문제 추가/삭제)
pub async fn manage_contest(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "contests");

    let user = match &auth_session.user {
        Some(u) => u,
        None => {
            return Html(state.tera.render("error.html", &context).unwrap());
        }
    };

    context.insert("current_user", user);

    // 대회 정보 조회
    let contest = sqlx::query_as::<_, ContestDetail>(
        r#"
        SELECT c.*, u.username as creator_username
        FROM contests c
        JOIN users u ON c.created_by = u.id
        WHERE c.id = ?
        "#
    )
    .bind(id)
    .fetch_one(&state.db_pool)
    .await;

    let contest = match contest {
        Ok(c) => c,
        Err(_) => {
            return Html(state.tera.render("error.html", &context).unwrap());
        }
    };

    // 권한 확인: 대회 생성자 또는 관리자만
    if contest.created_by != user.id && user.role != "admin" {
        context.insert("error", "대회 관리 권한이 없습니다.");
        return Html(state.tera.render("error.html", &context).unwrap());
    }

    // 대회 문제 목록
    let problems = sqlx::query_as::<_, ContestProblem>(
        r#"
        SELECT cp.id, cp.problem_id, cp.points, cp.problem_order,
               '' as problem_title
        FROM contest_problems cp
        WHERE cp.contest_id = ?
        ORDER BY cp.problem_order
        "#
    )
    .bind(id)
    .fetch_all(&state.db_pool)
    .await
    .unwrap_or_default();

    context.insert("contest", &contest);
    context.insert("problems", &problems);

    Html(state.tera.render("contest_manage.html", &context).unwrap())
}

// 대회에 문제 추가
pub async fn add_contest_problem(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(contest_id): Path<i64>,
    Form(form): Form<AddContestProblemForm>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    // 권한 확인
    let contest: Result<(i64,), _> = sqlx::query_as(
        "SELECT created_by FROM contests WHERE id = ? AND (created_by = ? OR ? = 'admin')"
    )
    .bind(contest_id)
    .bind(user.id)
    .bind(&user.role)
    .fetch_one(&state.db_pool)
    .await;

    if contest.is_err() {
        return (StatusCode::FORBIDDEN, "권한이 없습니다.").into_response();
    }

    // 문제 추가
    let result = sqlx::query(
        r#"
        INSERT INTO contest_problems (contest_id, problem_id, points, problem_order)
        VALUES (?, ?, ?, ?)
        "#
    )
    .bind(contest_id)
    .bind(form.problem_id)
    .bind(form.points)
    .bind(form.problem_order)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => Redirect::to(&format!("/contests/{}/manage", contest_id)).into_response(),
        Err(e) => {
            eprintln!("Failed to add problem to contest: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "문제 추가 실패").into_response()
        }
    }
}

// 대회에서 문제 삭제
pub async fn remove_contest_problem(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path((contest_id, problem_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    // 권한 확인
    let contest: Result<(i64,), _> = sqlx::query_as(
        "SELECT created_by FROM contests WHERE id = ? AND (created_by = ? OR ? = 'admin')"
    )
    .bind(contest_id)
    .bind(user.id)
    .bind(&user.role)
    .fetch_one(&state.db_pool)
    .await;

    if contest.is_err() {
        return (StatusCode::FORBIDDEN, "권한이 없습니다.").into_response();
    }

    // 문제 삭제
    let result = sqlx::query(
        "DELETE FROM contest_problems WHERE contest_id = ? AND id = ?"
    )
    .bind(contest_id)
    .bind(problem_id)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(_) => Redirect::to(&format!("/contests/{}/manage", contest_id)).into_response(),
        Err(e) => {
            eprintln!("Failed to remove problem from contest: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "문제 삭제 실패").into_response()
        }
    }
}

// 대회 중 문제 제출
pub async fn submit_contest_problem(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path((contest_id, problem_id)): Path<(i64, i64)>,
    Form(form): Form<SubmitForm>,
) -> impl IntoResponse {
    let user = match auth_session.user {
        Some(u) => u,
        None => return Redirect::to("/login").into_response(),
    };

    // 대회 참가자인지 확인
    let participant: Result<(i64,), _> = sqlx::query_as(
        "SELECT id FROM contest_participants WHERE contest_id = ? AND user_id = ?"
    )
    .bind(contest_id)
    .bind(user.id)
    .fetch_one(&state.db_pool)
    .await;

    if participant.is_err() {
        return (StatusCode::FORBIDDEN, "대회 참가자가 아닙니다.").into_response();
    }

    // 대회가 진행 중인지 확인
    let contest: Result<(String, String), _> = sqlx::query_as(
        "SELECT start_time, end_time FROM contests WHERE id = ?"
    )
    .bind(contest_id)
    .fetch_one(&state.db_pool)
    .await;

    let (start_time, end_time) = match contest {
        Ok(c) => c,
        Err(_) => return (StatusCode::NOT_FOUND, "대회를 찾을 수 없습니다.").into_response(),
    };

    let now = Utc::now();
    let start = match parse_datetime(&start_time) {
        Some(dt) => dt,
        None => return (StatusCode::BAD_REQUEST, "잘못된 대회 시작 시간 형식").into_response(),
    };
    let end = match parse_datetime(&end_time) {
        Some(dt) => dt,
        None => return (StatusCode::BAD_REQUEST, "잘못된 대회 종료 시간 형식").into_response(),
    };

    if now < start || now > end {
        return (StatusCode::FORBIDDEN, "대회가 진행 중이 아닙니다.").into_response();
    }

    // 제출 생성 (contest_id 포함)
    let result = sqlx::query(
        r#"
        INSERT INTO submissions (user_id, problem_id, contest_id, language, source_code, status)
        VALUES (?, ?, ?, ?, ?, 'PENDING')
        "#
    )
    .bind(user.id)
    .bind(problem_id)
    .bind(contest_id)
    .bind(&form.language)
    .bind(&form.source_code)
    .execute(&state.db_pool)
    .await;

    match result {
        Ok(result) => {
            let submission_id = result.last_insert_rowid();
            Redirect::to(&format!("/submissions/{}", submission_id)).into_response()
        }
        Err(e) => {
            eprintln!("Failed to submit: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "제출 실패").into_response()
        }
    }
}

// 대회 문제 페이지 (대회 중에만 접근 가능)
pub async fn contest_problem_detail(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path((contest_id, problem_id)): Path<(i64, i64)>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "contests");

    let user = match &auth_session.user {
        Some(u) => u,
        None => {
            return Html(state.tera.render("error.html", &context).unwrap());
        }
    };

    context.insert("current_user", user);

    // 대회 참가자인지 확인
    let participant: Result<(i64,), _> = sqlx::query_as(
        "SELECT id FROM contest_participants WHERE contest_id = ? AND user_id = ?"
    )
    .bind(contest_id)
    .bind(user.id)
    .fetch_one(&state.db_pool)
    .await;

    if participant.is_err() {
        context.insert("error", "대회 참가자가 아닙니다.");
        return Html(state.tera.render("error.html", &context).unwrap());
    }

    // 대회 정보 조회
    let contest = sqlx::query_as::<_, ContestDetail>(
        r#"
        SELECT c.*, u.username as creator_username
        FROM contests c
        JOIN users u ON c.created_by = u.id
        WHERE c.id = ?
        "#
    )
    .bind(contest_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap();

    // 문제가 대회에 포함되어 있는지 확인
    let contest_problem: Result<(i32, i32), _> = sqlx::query_as(
        "SELECT problem_order, points FROM contest_problems WHERE contest_id = ? AND problem_id = ?"
    )
    .bind(contest_id)
    .bind(problem_id)
    .fetch_one(&state.db_pool)
    .await;

    let (problem_order, points) = match contest_problem {
        Ok(p) => p,
        Err(_) => {
            context.insert("error", "대회에 포함되지 않은 문제입니다.");
            return Html(state.tera.render("error.html", &context).unwrap());
        }
    };

    // 문제 정보 로드 (기존 problem_detail 로직 재사용)
    context.insert("contest", &contest);
    context.insert("contest_id", &contest_id);
    context.insert("problem_order", &problem_order);
    context.insert("points", &points);
    context.insert("problem_id", &problem_id);

    Html(state.tera.render("contest_problem.html", &context).unwrap())
}
