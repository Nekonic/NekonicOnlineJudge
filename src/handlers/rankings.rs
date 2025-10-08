use axum::{
    extract::State,
    response::Html,
};
use axum_login::AuthSession;
use tera::Context;

use crate::{
    auth::Backend,
    error::AppError,
    models::{OrganizationRankingInfo, RankingEntry, RankingsQuery},
    AppState,
};

pub async fn rankings_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    axum::extract::Query(query): axum::extract::Query<RankingsQuery>,
) -> Result<Html<String>, AppError> {
    let user_type = query.user_type.clone();
    let organization_id = query.organization_id;
    let view_type = query.view_type.as_deref();
    let sort_by = query.sort_by.as_deref().unwrap_or("avg_rating");
    let sort_order = query.sort_order.as_deref().unwrap_or("desc");

    // 조직 정보 조회 (드롭다운용)
    let organizations_school: Vec<OrganizationRankingInfo> = sqlx::query_as(
        r#"
        SELECT o.id, o.name, o.type,
               COUNT(DISTINCT uo.user_id) as member_count,
               ROUND(COALESCE(AVG(us.rating), 1500.0), 1) as avg_rating,
               ROUND(COALESCE(SUM(us.rating), 0), 1) as total_rating,
               COALESCE(SUM(us.total_solved), 0) as total_solved,
               0 as rank
        FROM organizations o
        LEFT JOIN user_organizations uo ON o.id = uo.organization_id AND uo.status = 'active'
        LEFT JOIN user_stats us ON uo.user_id = us.user_id
        WHERE o.status = 'approved' AND o.type = 'school'
        GROUP BY o.id
        ORDER BY o.name
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    let organizations_company: Vec<OrganizationRankingInfo> = sqlx::query_as(
        r#"
        SELECT o.id, o.name, o.type,
               COUNT(DISTINCT uo.user_id) as member_count,
               ROUND(COALESCE(AVG(us.rating), 1500.0), 1) as avg_rating,
               ROUND(COALESCE(SUM(us.rating), 0), 1) as total_rating,
               COALESCE(SUM(us.total_solved), 0) as total_solved,
               0 as rank
        FROM organizations o
        LEFT JOIN user_organizations uo ON o.id = uo.organization_id AND uo.status = 'active'
        LEFT JOIN user_stats us ON uo.user_id = us.user_id
        WHERE o.status = 'approved' AND o.type = 'company'
        GROUP BY o.id
        ORDER BY o.name
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    let organizations_club: Vec<OrganizationRankingInfo> = sqlx::query_as(
        r#"
        SELECT o.id, o.name, o.type,
               COUNT(DISTINCT uo.user_id) as member_count,
               ROUND(COALESCE(AVG(us.rating), 1500.0), 1) as avg_rating,
               ROUND(COALESCE(SUM(us.rating), 0), 1) as total_rating,
               COALESCE(SUM(us.total_solved), 0) as total_solved,
               0 as rank
        FROM organizations o
        LEFT JOIN user_organizations uo ON o.id = uo.organization_id AND uo.status = 'active'
        LEFT JOIN user_stats us ON uo.user_id = us.user_id
        WHERE o.status = 'approved' AND o.type = 'club'
        GROUP BY o.id
        ORDER BY o.name
        "#
    )
    .fetch_all(&state.db_pool)
    .await?;

    let mut context = Context::new();
    context.insert("active_page", "rankings");
    context.insert("organizations_school", &organizations_school);
    context.insert("organizations_company", &organizations_company);
    context.insert("organizations_club", &organizations_club);
    context.insert("sort_by", sort_by);
    context.insert("sort_order", sort_order);
    context.insert("view_type", &view_type);

    // 조직 타입별 랭킹 보기
    if let Some(vtype) = view_type {
        let order_by_clause = match sort_by {
            "member_count" => "member_count",
            "total_rating" => "total_rating",
            "total_solved" => "total_solved",
            _ => "avg_rating",
        };
        let order_dir = if sort_order == "asc" { "ASC" } else { "DESC" };

        let org_type = match vtype {
            "school" => "school",
            "company" => "company",
            "club" => "club",
            _ => "school",
        };

        let sql = format!(
            r#"
            WITH org_stats AS (
                SELECT
                    o.id,
                    o.name,
                    o.type,
                    COUNT(DISTINCT uo.user_id) as member_count,
                    ROUND(COALESCE(AVG(us.rating), 1500.0), 1) as avg_rating,
                    ROUND(COALESCE(SUM(us.rating), 0), 1) as total_rating,
                    COALESCE(SUM(us.total_solved), 0) as total_solved
                FROM organizations o
                LEFT JOIN user_organizations uo ON o.id = uo.organization_id AND uo.status = 'active'
                LEFT JOIN user_stats us ON uo.user_id = us.user_id
                WHERE o.status = 'approved' AND o.type = ?
                GROUP BY o.id, o.name, o.type
            )
            SELECT
                id,
                name,
                type,
                member_count,
                avg_rating,
                total_rating,
                total_solved,
                ROW_NUMBER() OVER (ORDER BY {} {}, name ASC) as rank
            FROM org_stats
            ORDER BY {} {}, name ASC
            "#,
            order_by_clause, order_dir, order_by_clause, order_dir
        );

        let organization_rankings: Vec<OrganizationRankingInfo> = sqlx::query_as(&sql)
            .bind(org_type)
            .fetch_all(&state.db_pool)
            .await?;

        context.insert("view_type", vtype);
        context.insert("organization_rankings", &organization_rankings);
        context.insert("organization_id", &None::<i64>);
        context.insert("rankings", &Vec::<RankingEntry>::new());

        if let Some(user) = auth_session.user {
            context.insert("current_user", &user);
        }

        let html = state.tera.render("rankings.html", &context)?;
        return Ok(Html(html));
    }

    // 정렬 기준 결정 (사용자 랭킹용)
    let order_by_clause = match sort_by {
        "solved_count" => "COALESCE(us.total_solved, 0)",
        _ => "COALESCE(us.rating, 1500)",
    };
    let order_dir = if sort_order == "asc" { "ASC" } else { "DESC" };

    // 사용자 랭킹 조회
    let rankings: Vec<RankingEntry> = if let Some(org_id) = organization_id {
        // 특정 조직의 멤버만 조회
        let sql = format!(
            r#"
            SELECT
                ROW_NUMBER() OVER (ORDER BY {} {}, u.username ASC) as rank,
                u.username,
                COALESCE(us.rating, 1500) as rating,
                COALESCE(us.total_solved, 0) as solved_count,
                COALESCE(u.user_type, 'individual') as user_type
            FROM users u
            LEFT JOIN user_stats us ON u.id = us.user_id
            INNER JOIN user_organizations uo ON u.id = uo.user_id
            WHERE uo.organization_id = ? AND uo.status = 'active'
            ORDER BY {} {}, u.username ASC
            LIMIT 100
            "#,
            order_by_clause, order_dir, order_by_clause, order_dir
        );
        sqlx::query_as(&sql)
            .bind(org_id)
            .fetch_all(&state.db_pool)
            .await?
    } else {
        // 전체 사용자 조회
        let sql = format!(
            r#"
            SELECT
                ROW_NUMBER() OVER (ORDER BY {} {}, u.username ASC) as rank,
                u.username,
                COALESCE(us.rating, 1500) as rating,
                COALESCE(us.total_solved, 0) as solved_count,
                COALESCE(u.user_type, 'individual') as user_type
            FROM users u
            LEFT JOIN user_stats us ON u.id = us.user_id
            ORDER BY {} {}, u.username ASC
            LIMIT 100
            "#,
            order_by_clause, order_dir, order_by_clause, order_dir
        );
        sqlx::query_as(&sql)
            .fetch_all(&state.db_pool)
            .await?
    };

    context.insert("user_type", &user_type);
    context.insert("organization_id", &organization_id);
    context.insert("rankings", &rankings);

    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }

    let html = state.tera.render("rankings.html", &context)?;
    Ok(Html(html))
}
