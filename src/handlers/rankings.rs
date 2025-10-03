use axum::{
    extract::State,
    response::Html,
};
use axum_login::AuthSession;
use tera::Context;

use crate::{
    auth::Backend,
    error::AppError,
    models::{RankingEntry, RankingsQuery},
    AppState,
};

pub async fn rankings_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    axum::extract::Query(query): axum::extract::Query<RankingsQuery>,
) -> Result<Html<String>, AppError> {
    let user_type = query.user_type.clone();
    let sort_by = query.sort_by.as_deref().unwrap_or("rating");
    let sort_order = query.sort_order.as_deref().unwrap_or("desc");

    // 정렬 기준 결정
    let order_by_clause = match sort_by {
        "solved_count" => "COALESCE(us.total_solved, 0)",
        _ => "COALESCE(us.rating, 1500)", // 기본값: rating
    };

    let order_dir = if sort_order == "asc" { "ASC" } else { "DESC" };

    let sql = format!(
        "SELECT
            ROW_NUMBER() OVER (ORDER BY {} {}, u.username ASC) as rank,
            u.username,
            COALESCE(us.rating, 1500) as rating,
            COALESCE(us.total_solved, 0) as solved_count,
            COALESCE(u.user_type, 'individual') as user_type
         FROM users u
         LEFT JOIN user_stats us ON u.id = us.user_id
         WHERE COALESCE(u.user_type, 'individual') = ?
         ORDER BY {} {}, u.username ASC
         LIMIT 100",
        order_by_clause, order_dir, order_by_clause, order_dir
    );

    let rankings: Vec<RankingEntry> = sqlx::query_as(&sql)
        .bind(&user_type)
        .fetch_all(&state.db_pool)
        .await?;

    let mut context = Context::new();
    context.insert("active_page", "rankings");
    context.insert("user_type", &user_type);
    context.insert("sort_by", sort_by);
    context.insert("sort_order", sort_order);
    context.insert("rankings", &rankings);
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }

    let html = state.tera.render("rankings.html", &context)?;
    Ok(Html(html))
}

