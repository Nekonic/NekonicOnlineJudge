use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_login::AuthSession;
use crate::{
    auth::Backend,
    models::*,
    AppState,
};

/// 그룹 목록 조회
pub async fn list_organizations(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, Response> {
    let user = auth_session.user;

    let organizations: Vec<Organization> = sqlx::query_as(
        "SELECT * FROM organizations WHERE status = 'approved' ORDER BY created_at DESC"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let mut context = tera::Context::new();
    context.insert("current_user", &user);
    context.insert("active_page", "organizations");
    context.insert("organizations", &organizations);

    let rendered = state
        .tera
        .render("organizations_list.html", &context)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Html(rendered))
}

/// 그룹 상세 정보
pub async fn organization_detail(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(org_id): Path<i64>,
) -> Result<Html<String>, Response> {
    let user = auth_session.user;

    let organization: Organization = sqlx::query_as(
        "SELECT * FROM organizations WHERE id = ? AND status = 'approved'"
    )
    .bind(org_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| (StatusCode::NOT_FOUND, "그룹을 찾을 수 없습니다").into_response())?;

    let members: Vec<OrganizationMember> = sqlx::query_as(
        "SELECT uo.id, uo.user_id, u.username, uo.role, uo.status, uo.joined_at
         FROM user_organizations uo
         JOIN users u ON uo.user_id = u.id
         WHERE uo.organization_id = ? AND uo.status = 'active'
         ORDER BY uo.joined_at DESC"
    )
    .bind(org_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 사용자의 멤버십 상태 확인
    let is_member = if let Some(ref user) = user {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_organizations 
             WHERE organization_id = ? AND user_id = ? AND status = 'active'"
        )
        .bind(org_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    } else {
        false
    };

    // 가입 요청 대기 중인지 확인
    let has_pending_request = if let Some(ref user) = user {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM organization_join_requests 
             WHERE organization_id = ? AND user_id = ? AND status = 'pending'"
        )
        .bind(org_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    } else {
        false
    };

    // 사용자가 그룹 관리자인지 확인
    let is_group_admin = if let Some(ref user) = user {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_organizations
             WHERE organization_id = ? AND user_id = ? AND role = 'ADMIN' AND status = 'active'"
        )
        .bind(org_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    } else {
        false
    };

    let mut context = tera::Context::new();
    context.insert("current_user", &user);
    context.insert("active_page", "organizations");
    context.insert("organization", &organization);
    context.insert("members", &members);
    context.insert("is_member", &is_member);
    context.insert("has_pending_request", &has_pending_request);
    context.insert("is_group_admin", &is_group_admin);

    let rendered = state
        .tera
        .render("organization_detail.html", &context)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Html(rendered))
}

/// 사용자가 그룹 생성 (승인 필요)
pub async fn create_organization(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Form(form): Form<CreateOrganizationForm>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    let result = sqlx::query(
        "INSERT INTO organizations (name, type, description, status, created_by)
         VALUES (?, ?, ?, 'pending', ?)"
    )
    .bind(&form.name)
    .bind(&form.r#type)
    .bind(&form.description)
    .bind(user.id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 승인 대기 중이므로 내 그룹 페이지로 리다이렉트
    Ok(Redirect::to("/organizations/my"))
}

/// 그룹 가입 요청
pub async fn request_join_organization(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(org_id): Path<i64>,
    Form(form): Form<JoinOrganizationForm>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    // 이미 멤버인지 확인
    let is_member: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM user_organizations 
         WHERE organization_id = ? AND user_id = ?"
    )
    .bind(org_id)
    .bind(user.id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or(0);

    if is_member > 0 {
        return Err((StatusCode::BAD_REQUEST, "이미 그룹의 멤버입니다").into_response());
    }

    // 가입 요청 생성
    sqlx::query(
        "INSERT INTO organization_join_requests (organization_id, user_id, message)
         VALUES (?, ?, ?)
         ON CONFLICT(organization_id, user_id, status) DO NOTHING"
    )
    .bind(org_id)
    .bind(user.id)
    .bind(&form.message)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Redirect::to(&format!("/organizations/{}", org_id)))
}

/// 내 그룹 목록
pub async fn my_organizations(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
) -> Result<Html<String>, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    let organizations: Vec<Organization> = sqlx::query_as(
        "SELECT o.* FROM organizations o
         JOIN user_organizations uo ON o.id = uo.organization_id
         WHERE uo.user_id = ? AND uo.status = 'active'
         ORDER BY uo.joined_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    // 내가 생성한 그룹 (승인 대기 중)
    let pending_organizations: Vec<Organization> = sqlx::query_as(
        "SELECT * FROM organizations 
         WHERE created_by = ? AND status = 'pending'
         ORDER BY created_at DESC"
    )
    .bind(user.id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    let mut context = tera::Context::new();
    context.insert("current_user", &user);
    context.insert("active_page", "my_organizations");
    context.insert("organizations", &organizations);
    context.insert("pending_organizations", &pending_organizations);

    let rendered = state
        .tera
        .render("my_organizations.html", &context)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Html(rendered))
}

/// 그룹 삭제 (시스템 관리자만)
pub async fn delete_organization(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(org_id): Path<i64>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    // 시스템 관리자 권한 확인
    if !user.is_admin() {
        return Err((StatusCode::FORBIDDEN, "시스템 관리자 권한이 필요합니다").into_response());
    }

    // 그룹 삭제 (CASCADE로 관련 데이터도 삭제됨)
    sqlx::query("DELETE FROM organizations WHERE id = ?")
        .bind(org_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Redirect::to("/organizations"))
}

/// 그룹 멤버를 그룹 관리자로 승격
pub async fn promote_to_group_admin(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path((org_id, member_id)): Path<(i64, i64)>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    // 시스템 관리자이거나 그룹 관리자인지 확인
    let is_authorized = if user.is_admin() {
        true
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_organizations
             WHERE organization_id = ? AND user_id = ? AND role = 'ADMIN' AND status = 'active'"
        )
        .bind(org_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    };

    if !is_authorized {
        return Err((StatusCode::FORBIDDEN, "권한이 없습니다").into_response());
    }

    // 멤버를 관리자로 승격
    sqlx::query(
        "UPDATE user_organizations SET role = 'ADMIN'
         WHERE organization_id = ? AND user_id = ?"
    )
    .bind(org_id)
    .bind(member_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Redirect::to(&format!("/organizations/{}", org_id)))
}

/// 그룹 멤버를 일반 멤버로 강등
pub async fn demote_to_member(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path((org_id, member_id)): Path<(i64, i64)>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    // 시스템 관리자이거나 그룹 관리자인지 확인
    let is_authorized = if user.is_admin() {
        true
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_organizations
             WHERE organization_id = ? AND user_id = ? AND role = 'ADMIN' AND status = 'active'"
        )
        .bind(org_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    };

    if !is_authorized {
        return Err((StatusCode::FORBIDDEN, "권한이 없습니다").into_response());
    }

    // 관리자를 일반 멤버로 강등
    sqlx::query(
        "UPDATE user_organizations SET role = 'MEMBER'
         WHERE organization_id = ? AND user_id = ?"
    )
    .bind(org_id)
    .bind(member_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Redirect::to(&format!("/organizations/{}", org_id)))
}

/// 그룹 멤버 추방
pub async fn remove_member(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path((org_id, member_id)): Path<(i64, i64)>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    // 시스템 관리자이거나 그룹 관리자인지 확인
    let is_authorized = if user.is_admin() {
        true
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_organizations
             WHERE organization_id = ? AND user_id = ? AND role = 'ADMIN' AND status = 'active'"
        )
        .bind(org_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    };

    if !is_authorized {
        return Err((StatusCode::FORBIDDEN, "권한이 없습니다").into_response());
    }

    // 멤버 삭제
    sqlx::query(
        "DELETE FROM user_organizations
         WHERE organization_id = ? AND user_id = ?"
    )
    .bind(org_id)
    .bind(member_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Redirect::to(&format!("/organizations/{}", org_id)))
}

/// 그룹에 사용자 초대 (그룹 관리자 기능)
pub async fn invite_member(
    State(state): State<AppState>,
    auth_session: AuthSession<Backend>,
    Path(org_id): Path<i64>,
    Form(form): Form<AddMemberForm>,
) -> Result<Redirect, Response> {
    let user = auth_session.user.ok_or_else(|| {
        (StatusCode::UNAUTHORIZED, "로그인이 필요합니다").into_response()
    })?;

    // 시스템 관리자이거나 그룹 관리자인지 확인
    let is_authorized = if user.is_admin() {
        true
    } else {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM user_organizations
             WHERE organization_id = ? AND user_id = ? AND role = 'ADMIN' AND status = 'active'"
        )
        .bind(org_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    };

    if !is_authorized {
        return Err((StatusCode::FORBIDDEN, "권한이 없습니다").into_response());
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

    // 멤버 추가
    sqlx::query(
        "INSERT INTO user_organizations (user_id, organization_id, role, added_by)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, organization_id) DO UPDATE SET
         role = excluded.role,
         status = 'active'"
    )
    .bind(target_user_id)
    .bind(org_id)
    .bind(&role)
    .bind(user.id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response())?;

    Ok(Redirect::to(&format!("/organizations/{}", org_id)))
}
