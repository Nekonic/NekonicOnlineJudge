use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form, Json,
};
use axum_login::AuthSession;
use serde_json::json;
use crate::{
    auth::Backend,
    models::*,
    AppState,
};

/// 관리자 대시보드
pub async fn admin_dashboard(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    // 대기 중인 그룹 요청 수
    let pending_org_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organizations WHERE status = 'pending'"
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 대기 중인 가입 요청 수
    let pending_join_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM organization_join_requests WHERE status = 'pending'"
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 최근 관리자 액션
    let recent_actions: Vec<AdminAction> = sqlx::query_as(
        "SELECT a.*, u.username as admin_username
         FROM admin_actions a
         JOIN users u ON a.admin_id = u.id
         ORDER BY a.created_at DESC
         LIMIT 10"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let mut context = tera::Context::new();
    context.insert("current_user", &user);
    context.insert("active_page", "admin");
    context.insert("pending_org_count", &pending_org_count);
    context.insert("pending_join_count", &pending_join_count);
    context.insert("recent_actions", &recent_actions);

    let rendered = state
        .tera
        .render("admin_dashboard.html", &context)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Html(rendered))
}

/// 그룹 승인 대기 목록
pub async fn pending_organizations(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    let organizations: Vec<Organization> = sqlx::query_as(
        "SELECT * FROM organizations WHERE status = 'pending' ORDER BY created_at DESC"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let mut context = tera::Context::new();
    context.insert("current_user", &user);
    context.insert("active_page", "admin_orgs");
    context.insert("organizations", &organizations);

    let rendered = state
        .tera
        .render("admin_pending_orgs.html", &context)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Html(rendered))
}

/// 그룹 승인/거부
pub async fn review_organization(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(org_id): Path<i64>,
    Form(form): Form<ReviewRequestForm>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    let new_status = match form.action.as_str() {
        "approve" => "approved",
        "reject" => "rejected",
        _ => return Err((StatusCode::BAD_REQUEST, "잘못된 액션입니다").into_response()),
    };

    sqlx::query(
        "UPDATE organizations
         SET status = ?, approved_by = ?, approved_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(new_status)
    .bind(user.id)
    .bind(org_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 관리자 액션 로그 기록
    sqlx::query(
        "INSERT INTO admin_actions (admin_id, action_type, target_type, target_id, details)
         VALUES (?, ?, 'organization', ?, ?)"
    )
    .bind(user.id)
    .bind(format!("organization_{}", form.action))
    .bind(org_id)
    .bind(format!("Organization {} {}", org_id, new_status))
    .execute(&state.db_pool)
    .await
    .ok();

    Ok(Redirect::to("/admin/organizations/pending"))
}

/// 그룹 가입 요청 목록
pub async fn pending_join_requests(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    let requests: Vec<OrganizationJoinRequest> = sqlx::query_as(
        "SELECT jr.*, u.username
         FROM organization_join_requests jr
         JOIN users u ON jr.user_id = u.id
         WHERE jr.status = 'pending'
         ORDER BY jr.requested_at DESC"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let mut context = tera::Context::new();
    context.insert("current_user", &user);
    context.insert("active_page", "admin_joins");
    context.insert("requests", &requests);

    let rendered = state
        .tera
        .render("admin_pending_joins.html", &context)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Html(rendered))
}

/// 그룹 가입 요청 승인/거부
pub async fn review_join_request(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(request_id): Path<i64>,
    Form(form): Form<ReviewRequestForm>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    // 요청 정보 가져오기
    let request: (i64, i64) = sqlx::query_as(
        "SELECT organization_id, user_id FROM organization_join_requests WHERE id = ?"
    )
    .bind(request_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let (org_id, user_id) = request;

    let new_status = match form.action.as_str() {
        "approve" => {
            // 멤버 추가
            sqlx::query(
                "INSERT INTO user_organizations (user_id, organization_id, role, added_by)
                 VALUES (?, ?, 'MEMBER', ?)"
            )
            .bind(user_id)
            .bind(org_id)
            .bind(user.id)
            .execute(&state.db_pool)
            .await
            .ok();

            "approved"
        }
        "reject" => "rejected",
        _ => return Err((StatusCode::BAD_REQUEST, "잘못된 액션입니다").into_response()),
    };

    // 요청 상태 업데이트
    sqlx::query(
        "UPDATE organization_join_requests
         SET status = ?, reviewed_by = ?, reviewed_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(new_status)
    .bind(user.id)
    .bind(request_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 관리자 액션 로그
    sqlx::query(
        "INSERT INTO admin_actions (admin_id, action_type, target_type, target_id, details)
         VALUES (?, ?, 'join_request', ?, ?)"
    )
    .bind(user.id)
    .bind(format!("join_request_{}", form.action))
    .bind(request_id)
    .bind(format!("Join request {} {}", request_id, new_status))
    .execute(&state.db_pool)
    .await
    .ok();

    Ok(Redirect::to("/admin/join-requests/pending"))
}

/// 관리자가 직접 그룹 생성
pub async fn create_organization_admin(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Form(form): Form<CreateOrganizationForm>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    let result = sqlx::query(
        "INSERT INTO organizations (name, type, description, status, created_by, approved_by, approved_at)
         VALUES (?, ?, ?, 'approved', ?, ?, CURRENT_TIMESTAMP)"
    )
    .bind(&form.name)
    .bind(&form.r#type)
    .bind(&form.description)
    .bind(user.id)
    .bind(user.id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let org_id = result.last_insert_rowid();

    // 관리자 액션 로그
    sqlx::query(
        "INSERT INTO admin_actions (admin_id, action_type, target_type, target_id, details)
         VALUES (?, 'create_organization', 'organization', ?, ?)"
    )
    .bind(user.id)
    .bind(org_id)
    .bind(format!("Created organization: {}", form.name))
    .execute(&state.db_pool)
    .await
    .ok();

    Ok(Redirect::to(&format!("/organizations/{}", org_id)))
}

/// 관리자가 그룹에 사용자 추가
pub async fn add_member_to_organization(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(org_id): Path<i64>,
    Form(form): Form<AddMemberForm>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    // username으로 사용자 ID 조회
    let target_user_id: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM users WHERE username = ?"
    )
    .bind(&form.username)
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let target_user_id = target_user_id.ok_or_else(|| {
        (StatusCode::NOT_FOUND, "사용자를 찾을 수 없습니다").into_response()
    })?;

    let role = form.role.unwrap_or_else(|| "MEMBER".to_string());

    sqlx::query(
        "INSERT INTO user_organizations (user_id, organization_id, role, added_by)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, organization_id) DO UPDATE SET
         role = excluded.role,
         added_by = excluded.added_by"
    )
    .bind(target_user_id)
    .bind(org_id)
    .bind(&role)
    .bind(user.id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 관리자 액션 로그
    sqlx::query(
        "INSERT INTO admin_actions (admin_id, action_type, target_type, target_id, details)
         VALUES (?, 'add_member', 'organization', ?, ?)"
    )
    .bind(user.id)
    .bind(org_id)
    .bind(format!("Added user {} to organization {}", form.username, org_id))
    .execute(&state.db_pool)
    .await
    .ok();

    Ok(Redirect::to(&format!("/organizations/{}", org_id)))
}

/// 사용자를 관리자로 승격
pub async fn promote_to_admin(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(user_id): Path<i64>,
) -> Result<Json<serde_json::Value>, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "관리자 권한이 필요합니다").into_response());
    }

    sqlx::query("UPDATE users SET role = 'admin' WHERE id = ?")
        .bind(user_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 관리자 액션 로그
    sqlx::query(
        "INSERT INTO admin_actions (admin_id, action_type, target_type, target_id, details)
         VALUES (?, 'promote_admin', 'user', ?, ?)"
    )
    .bind(user.id)
    .bind(user_id)
    .bind(format!("Promoted user {} to admin", user_id))
    .execute(&state.db_pool)
    .await
    .ok();

    Ok(Json(json!({
        "success": true,
        "message": "사용자를 관리자로 승격했습니다"
    })))
}
