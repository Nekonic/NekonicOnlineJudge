use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
    Form,
};
use axum_login::AuthSession;
use tera::Context;

use crate::{
    auth::Backend,
    error::AppError,
    judge,
    models::{ProblemStatusData, SubmissionDetailData, SubmissionDetailRow, SubmissionRow, SubmitForm, TestcaseResultRow},
    AppState,
};

use super::problems::load_problem_detail;

pub async fn submit_solution(
    Path(problem_id): Path<i64>,
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Form(form): Form<SubmitForm>,
) -> Result<Redirect, AppError> {
    let user = auth_session.user.ok_or(AppError::Unauthorized)?;

    let submission_id: i64 = sqlx::query_scalar(
        "INSERT INTO submissions (user_id, problem_id, language, source_code, status)
         VALUES (?, ?, ?, ?, 'PENDING') RETURNING id",
    )
    .bind(user.id)
    .bind(problem_id)
    .bind(&form.language)
    .bind(&form.source_code)
    .fetch_one(&state.db_pool)
    .await?;

    tokio::spawn(async move {
        let judge_request = judge::JudgeRequest {
            submission_id,
            language: form.language,
            source_code: form.source_code,
            problem_id,
        };

        if let Ok(result) = judge::judge_submission(judge_request).await {
            let _ = sqlx::query(
                "UPDATE submissions SET status = ?, score = ?, execution_time = ?,
                 memory_usage = ?, compile_message = ?, runtime_error_type = ?,
                 runtime_error_message = ?, total_testcases = ?, passed_testcases = ?,
                 judged_at = CURRENT_TIMESTAMP WHERE id = ?",
            )
            .bind(&result.status)
            .bind(result.score)
            .bind(result.execution_time)
            .bind(result.memory_usage)
            .bind(&result.compile_message)
            .bind(&result.runtime_error_type)
            .bind(&result.runtime_error_message)
            .bind(result.total_testcases)
            .bind(result.passed_testcases)
            .bind(submission_id)
            .execute(&state.db_pool)
            .await;

            for testcase in result.testcase_results {
                let _ = sqlx::query(
                    "INSERT INTO testcase_results
                     (submission_id, testcase_number, status, execution_time, memory_usage, error_message, expected_output, actual_output)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(submission_id)
                .bind(testcase.testcase_number)
                .bind(&testcase.status)
                .bind(testcase.execution_time)
                .bind(testcase.memory_usage)
                .bind(&testcase.error_message)
                .bind(&testcase.expected_output)
                .bind(&testcase.actual_output)
                .execute(&state.db_pool)
                .await;
            }

            if let Some(compile_errors) = result.compile_errors {
                for error in compile_errors {
                    let _ = sqlx::query(
                        "INSERT INTO compile_errors
                         (submission_id, line_number, column_number, error_type, error_message)
                         VALUES (?, ?, ?, ?, ?)",
                    )
                    .bind(submission_id)
                    .bind(error.line_number)
                    .bind(error.column_number)
                    .bind(&error.error_type)
                    .bind(&error.error_message)
                    .execute(&state.db_pool)
                    .await;
                }
            }

            // 대회 제출인 경우 순위 업데이트
            if let Ok(Some((contest_id, user_id))) = sqlx::query_as::<_, (i64, i64)>(
                "SELECT contest_id, user_id FROM submissions WHERE id = ? AND contest_id IS NOT NULL"
            )
            .bind(submission_id)
            .fetch_optional(&state.db_pool)
            .await
            {
                // 대회 순위 업데이트
                let _ = crate::contest_scoring::update_standings(&state.db_pool, contest_id, user_id).await;
            }
        }
    });

    Ok(Redirect::to(&format!("/submissions/{}", submission_id)))
}

pub async fn problem_status(
    Path(problem_id): Path<i64>,
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, AppError> {
    let problem = load_problem_detail(problem_id, &state).await?;

    let submissions: Vec<SubmissionRow> = sqlx::query_as(
        "SELECT s.id, u.username, s.language, s.status, s.score,
                s.execution_time, s.memory_usage,
                s.created_at as submitted_at
         FROM submissions s
         JOIN users u ON s.user_id = u.id
         WHERE s.problem_id = ?
         ORDER BY s.created_at DESC
         LIMIT 50",
    )
    .bind(problem_id)
    .fetch_all(&state.db_pool)
    .await?;

    let problem_status = ProblemStatusData {
        id: problem_id,
        title: problem.meta.title,
        submissions,
    };

    let mut context = Context::new();
    context.insert("active_page", "problems");
    context.insert("problem_status", &problem_status);
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }

    let html = state.tera.render("problem_status.html", &context)?;
    Ok(Html(html))
}

pub async fn submission_detail(
    Path(submission_id): Path<i64>,
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, AppError> {
    let submission: SubmissionDetailRow = sqlx::query_as(
        "SELECT s.id, s.problem_id, u.username, s.language, s.status, s.score,
                s.execution_time, s.memory_usage, s.compile_message,
                s.runtime_error_type, s.runtime_error_message,
                s.total_testcases, s.passed_testcases,
                datetime(s.created_at) as submitted_at,
                datetime(s.judged_at) as judged_at
         FROM submissions s
         JOIN users u ON s.user_id = u.id
         WHERE s.id = ?",
    )
    .bind(submission_id)
    .fetch_optional(&state.db_pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let testcase_results: Vec<TestcaseResultRow> = sqlx::query_as(
        "SELECT testcase_number, status, execution_time, memory_usage, error_message
         FROM testcase_results
         WHERE submission_id = ?
         ORDER BY testcase_number",
    )
    .bind(submission_id)
    .fetch_all(&state.db_pool)
    .await?;

    let submission_detail = SubmissionDetailData {
        submission,
        testcase_results,
    };

    let mut context = Context::new();
    context.insert("active_page", "problems");
    context.insert("submission_detail", &submission_detail);
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }

    let html = state.tera.render("submission_detail.html", &context)?;
    Ok(Html(html))
}
