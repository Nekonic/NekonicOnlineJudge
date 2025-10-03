use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_login::AuthSession;
use tera::Context;

use crate::{auth::Backend, AppState};

pub async fn contests_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "contests");
    context.insert("contests", &Vec::<serde_json::Value>::new());
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }
    Html(state.tera.render("contests_list.html", &context).unwrap())
}

