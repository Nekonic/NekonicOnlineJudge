use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

use crate::{handlers, AppState};

pub fn create_router() -> Router<AppState> {
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
        // Static files
        .nest_service("/static", ServeDir::new("static"))
}
