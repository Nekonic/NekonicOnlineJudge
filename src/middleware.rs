use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_login::AuthSession;
use crate::auth::{Backend, User};

/// 관리자 권한 체크 미들웨어
pub async fn require_admin(
    auth_session: AuthSession<Backend>,
    request: Request,
    next: Next,
) -> Response {
    match auth_session.user {
        Some(user) if user.is_admin() => next.run(request).await,
        Some(_) => (StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response(),
        None => (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response(),
    }
}

/// 로그인 확인 미들웨어
pub async fn require_auth(
    auth_session: AuthSession<Backend>,
    request: Request,
    next: Next,
) -> Response {
    match auth_session.user {
        Some(_) => next.run(request).await,
        None => (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response(),
    }
}

