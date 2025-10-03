use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_login::AuthSession;
use gray_matter::{engine::YAML, Matter, ParsedEntity};
use pulldown_cmark::{html, Parser};
use tera::Context;

use crate::{
    auth::Backend,
    error::AppError,
    models::{FrontMatter, ProblemDetail, ProblemListItem, ProblemMeta, ProblemStats},
    AppState,
};

#[axum::debug_handler]
pub async fn problems_list(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, AppError> {
    let problems = get_problems_from_fs(&state).await?;

    let mut context = Context::new();
    context.insert("active_page", "problems");
    context.insert("problems", &problems);
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }

    let html = state.tera.render("problems_list.html", &context)?;
    Ok(Html(html))
}

async fn get_problems_from_fs(state: &AppState) -> Result<Vec<ProblemListItem>, AppError> {
    let matter = Matter::<YAML>::new();
    let mut problems = Vec::new();

    let mut thousand_entries = tokio::fs::read_dir("problems").await?;

    while let Some(thousand_folder) = thousand_entries.next_entry().await? {
        if !thousand_folder.file_type().await?.is_dir() {
            continue;
        }

        let mut problem_entries = tokio::fs::read_dir(thousand_folder.path()).await?;

        while let Some(problem_folder) = problem_entries.next_entry().await? {
            if !problem_folder.file_type().await?.is_dir() {
                continue;
            }

            let problem_id = problem_folder
                .file_name()
                .to_string_lossy()
                .parse::<u32>()
                .ok();

            if problem_id.is_none() {
                continue;
            }
            let problem_id = problem_id.unwrap();

            let md_path = problem_folder.path().join(format!("{}.md", problem_id));
            if !md_path.exists() {
                continue;
            }

            let content = tokio::fs::read_to_string(&md_path).await?;
            let parsed: ParsedEntity = matter.parse(&content);

            #[derive(serde::Deserialize)]
            struct ProblemMeta {
                title: String,
            }

            if let Some(data) = parsed.data {
                if let Ok(meta) = data.deserialize::<ProblemMeta>() {
                    let problem_stats: Option<ProblemStats> = sqlx::query_as(
                        "SELECT problem_id, total_submissions, accepted_submissions, acceptance_rate,
                                avg_execution_time, avg_memory_usage
                         FROM submission_stats WHERE problem_id = ?",
                    )
                    .bind(problem_id as i64)
                    .fetch_optional(&state.db_pool)
                    .await?;

                    let accuracy = if let Some(stats) = problem_stats {
                        stats.acceptance_rate.unwrap_or(0.0)
                    } else {
                        0.0
                    };

                    problems.push(ProblemListItem {
                        id: problem_id,
                        title: meta.title,
                        accuracy: (accuracy * 10.0).round() / 10.0,
                    });
                }
            }
        }
    }

    problems.sort_by_key(|p| p.id);
    Ok(problems)
}

pub async fn problem_detail(
    Path(id): Path<u32>,
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, AppError> {
    let problem = load_problem_detail(id, &state).await?;

    let mut context = Context::new();
    context.insert("active_page", "problems");
    context.insert("problem", &problem);
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }

    let html = state.tera.render("problem.html", &context)?;
    Ok(Html(html))
}

pub async fn load_problem_detail(id: u32, state: &AppState) -> Result<ProblemDetail, AppError> {
    // 1001 -> 001000 폴더 (1000단위로 내림)
    let folder_num = (id / 1000) * 1000;
    let problem_path = std::path::Path::new("./problems")
        .join(format!("{:06}", folder_num))
        .join(id.to_string())
        .join(format!("{}.md", id));

    let content = tokio::fs::read_to_string(&problem_path)
        .await
        .map_err(|_| AppError::ProblemNotFound)?;

    let matter = Matter::<YAML>::new();
    let parsed = matter.parse(&content);

    let front_matter = parsed.data.ok_or(AppError::InvalidProblemFormat)?;

    let meta: FrontMatter = front_matter.deserialize()?;

    let problem_stats: Option<ProblemStats> = sqlx::query_as(
        "SELECT problem_id, total_submissions, accepted_submissions, acceptance_rate,
                avg_execution_time, avg_memory_usage
         FROM submission_stats WHERE problem_id = ?",
    )
    .bind(id as i64)
    .fetch_optional(&state.db_pool)
    .await?;

    let (total_submits, correct_submits, accuracy) = if let Some(stats) = problem_stats {
        (
            stats.total_submissions,
            stats.accepted_submissions,
            format!("{:.1}%", stats.acceptance_rate.unwrap_or(0.0)),
        )
    } else {
        (0, 0, "0.0%".to_string())
    };

    let parser = Parser::new(&parsed.content);
    let mut html_content = String::new();
    html::push_html(&mut html_content, parser);

    // 예제 입출력은 일단 비워둠
    let example_inputs = Vec::new();
    let example_outputs = Vec::new();

    Ok(ProblemDetail {
        id,
        meta: ProblemMeta {
            title: meta.title,
            time_limit: meta.time_limit,
            memory_limit: meta.memory_limit,
            tags: meta.tags,
        },
        content: html_content,
        example_inputs,
        example_outputs,
        total_submits,
        correct_submits,
        accuracy,
    })
}
