use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    Form,
};
use axum_login::AuthSession;
use tera::Context;

use crate::{auth::Backend, models::{LoginForm, RegisterForm}, AppState};

pub async fn login_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "login");
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }
    Html(state.tera.render("login.html", &context).unwrap())
}

pub async fn login_action(
    mut auth_session: AuthSession<Backend>,
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> impl IntoResponse {
    let creds = crate::auth::Credentials {
        username: form.username,
        password: form.password,
    };

    let user = match auth_session.authenticate(creds).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let mut context = Context::new();
            context.insert("active_page", "login");
            context.insert("error", "아이디 또는 비밀번호가 올바르지 않습니다.");
            return Html(state.tera.render("login.html", &context).unwrap());
        }
        Err(_) => {
            let mut context = Context::new();
            context.insert("active_page", "login");
            context.insert("error", "로그인 처리 중 오류가 발생했습니다.");
            return Html(state.tera.render("login.html", &context).unwrap());
        }
    };

    if auth_session.login(&user).await.is_err() {
        let mut context = Context::new();
        context.insert("active_page", "login");
        context.insert("error", "세션 생성 중 오류가 발생했습니다.");
        return Html(state.tera.render("login.html", &context).unwrap());
    }

    Html("<script>window.location.href='/';</script>".to_string())
}

pub async fn register_page(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> impl IntoResponse {
    let mut context = Context::new();
    context.insert("active_page", "register");
    if let Some(user) = auth_session.user {
        context.insert("current_user", &user);
    }
    Html(state.tera.render("register.html", &context).unwrap())
}

pub async fn register_action(
    State(state): State<AppState>,
    Form(form): Form<RegisterForm>,
) -> impl IntoResponse {
    if form.password.len() < 8 {
        let mut context = Context::new();
        context.insert("active_page", "register");
        context.insert("error", "비밀번호는 8자 이상이어야 합니다.");
        return Html(state.tera.render("register.html", &context).unwrap());
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(form.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    let result = sqlx::query("INSERT INTO users (username, password_hash) VALUES (?, ?)")
        .bind(&form.username)
        .bind(&password_hash)
        .execute(&state.db_pool)
        .await;

    match result {
        Ok(_) => Html(
            "<script>alert('회원가입이 완료되었습니다.'); window.location.href='/login';</script>"
                .to_string(),
        ),
        Err(_) => {
            let mut context = Context::new();
            context.insert("active_page", "register");
            context.insert("error", "이미 사용 중인 아이디입니다.");
            Html(state.tera.render("register.html", &context).unwrap())
        }
    }
}

pub async fn logout_action(mut auth_session: AuthSession<Backend>) -> impl IntoResponse {
    auth_session.logout().await.ok();
    axum::response::Redirect::to("/")
}
