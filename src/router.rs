use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use tower_http::services::ServeDir;

use crate::{handlers, AppState, middleware as app_middleware};

pub fn create_router() -> Router<AppState> {
    // 관리자 전용 라우트
    let admin_routes = Router::new()
        .route("/admin", get(handlers::admin_dashboard))
        .route("/admin/organizations/pending", get(handlers::pending_organizations))
        .route("/admin/organizations/:id/review", post(handlers::review_organization))
        .route("/admin/organizations/create", post(handlers::create_organization_admin))
        .route("/admin/organizations/:id/members/add", post(handlers::add_member_to_organization))
        .route("/admin/join-requests/pending", get(handlers::pending_join_requests))
        .route("/admin/join-requests/:id/review", post(handlers::review_join_request))
        .route("/admin/users/:id/promote", post(handlers::promote_to_admin))
        .route("/admin/organizations/:id/delete", post(handlers::delete_organization))
        .layer(middleware::from_fn(app_middleware::require_admin));

    // 인증 필요 라우트 (그룹 관리 포함)
    let auth_required_routes = Router::new()
        .route("/organizations/create", post(handlers::create_organization))
        .route("/organizations/:id/join", post(handlers::request_join_organization))
        .route("/organizations/my", get(handlers::my_organizations))
        .route("/organizations/:org_id/members/:member_id/promote", post(handlers::promote_to_group_admin))
        .route("/organizations/:org_id/members/:member_id/demote", post(handlers::demote_to_member))
        .route("/organizations/:org_id/members/:member_id/remove", post(handlers::remove_member))
        .route("/organizations/:org_id/members/invite", post(handlers::invite_member))
        .layer(middleware::from_fn(app_middleware::require_auth));

    Router::new()
        // Home
        .route("/", get(handlers::root))
        .route("/learn", get(handlers::learn_page))
        // Problems
        .route("/problems", get(handlers::problems_list))
        .route("/problems/:id", get(handlers::problem_detail))
        .route("/problems/:id/submit", post(handlers::submit_solution))
        .route("/problems/:id/status", get(handlers::problem_status))
        // Submissions
        .route("/submissions/:id", get(handlers::submission_detail))
        // Auth
        .route("/login", get(handlers::login_page).post(handlers::login_action))
        .route("/register", get(handlers::register_page).post(handlers::register_action))
        .route("/logout", get(handlers::logout_action))
        // Rankings
        .route("/rankings", get(handlers::rankings_page))
        // Contests
        .route("/contests", get(handlers::contests_page))
        // Organizations (public routes)
        .route("/organizations", get(handlers::list_organizations))
        .route("/organizations/:id", get(handlers::organization_detail))
        // Merge protected routes
        .merge(admin_routes)
        .merge(auth_required_routes)
        // Static files
        .nest_service("/static", ServeDir::new("static"))
}
