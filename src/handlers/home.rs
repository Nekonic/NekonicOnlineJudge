use axum::{
    extract::State,
    response::{Html, IntoResponse},
};
use axum_login::AuthSession;
use tera::Context;

use crate::{auth::Backend, AppState};

pub async fn root(State(state): State<AppState>, auth_session: AuthSession<Backend>) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "home");
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }
    Html(state.tera.render("index.html", &context).unwrap())
}

pub async fn learn_page(State(state): State<AppState>, auth_session: AuthSession<Backend>) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "learn");
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }
    Html(state.tera.render("learn.html", &context).unwrap())
}

