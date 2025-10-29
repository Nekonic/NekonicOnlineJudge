use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect, IntoResponse},
    Form,
};
use axum_login::AuthSession;
use tera::Context;
use crate::{AppState, models::*, auth::Backend};

// 게시판 목록 조회
pub async fn boards_list(
    State(state): State<AppState>,
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let boards = sqlx::query_as::<_, Board>(
        "SELECT id, name, board_type, organization_id, description, created_at FROM boards ORDER BY
         CASE board_type
             WHEN 'announcement' THEN 1
             WHEN 'qna' THEN 2
             WHEN 'free' THEN 3
             ELSE 4
         END"
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        eprintln!("Database error in boards_list: {}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e))
    })?;

    let mut context = Context::new();
    context.insert("current_user", &auth.user);
    context.insert("active_page", "boards");
    context.insert("boards", &boards);

    let html = state.tera.render("boards_list.html", &context)
        .map_err(|e| {
            eprintln!("Template render error in boards_list: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", e))
        })?;

    Ok(Html(html))
}

// 특정 게시판의 게시글 목록
pub async fn board_posts(
    State(state): State<AppState>,
    Path(board_id): Path<i64>,
    Query(query): Query<BoardsQuery>,
    auth: AuthSession<Backend>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    // 게시판 정보 조회
    let board = sqlx::query_as::<_, Board>(
        "SELECT id, name, board_type, organization_id, description, created_at FROM boards WHERE id = ?"
    )
    .bind(board_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        eprintln!("Board not found error: {}", e);
        (axum::http::StatusCode::NOT_FOUND, "게시판을 찾을 수 없습니다".to_string())
    })?;

    let page = query.page.unwrap_or(1);
    let per_page = 20;
    let offset = (page - 1) * per_page;

    // 게시글 목록 조회 (통계 포함)
    let posts = sqlx::query_as::<_, PostWithStats>(
        "SELECT p.id, p.board_id, p.user_id, u.username, p.title,
                p.problem_id, p.contest_id, p.is_pinned, p.is_locked,
                p.view_count, p.created_at,
                COALESCE(COUNT(DISTINCT c.id), 0) as comment_count,
                COALESCE(COUNT(DISTINCT pl.id), 0) as like_count
         FROM posts p
         JOIN users u ON p.user_id = u.id
         LEFT JOIN comments c ON p.id = c.post_id
         LEFT JOIN post_likes pl ON p.id = pl.post_id
         WHERE p.board_id = ?
         GROUP BY p.id, p.board_id, p.user_id, u.username, p.title,
                  p.problem_id, p.contest_id, p.is_pinned, p.is_locked,
                  p.view_count, p.created_at
         ORDER BY p.is_pinned DESC, p.created_at DESC
         LIMIT ? OFFSET ?"
    )
    .bind(board_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        eprintln!("Posts fetch error: {}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    // 전체 게시글 수
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM posts WHERE board_id = ?"
    )
    .bind(board_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        eprintln!("Count query error: {}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
    })?;

    let total_pages = (total.0 + per_page - 1) / per_page;

    let mut context = Context::new();
    context.insert("current_user", &auth.user);
    context.insert("active_page", "boards");
    context.insert("board", &board);
    context.insert("posts", &posts);
    context.insert("page", &page);
    context.insert("total_pages", &total_pages);
    context.insert("search", &query.search);

    let html = state.tera.render("board_posts.html", &context)
        .map_err(|e| {
            eprintln!("Template render error in board_posts: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", e))
        })?;

    Ok(Html(html))
}

// 게시글 작성 폼
pub async fn new_post_form(
    State(state): State<AppState>,
    Path(board_id): Path<i64>,
    Query(query): Query<NewPostQuery>,
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/login").into_response());
    }

    let board = sqlx::query_as::<_, Board>(
        "SELECT * FROM boards WHERE id = ?"
    )
    .bind(board_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| {
        eprintln!("Board fetch error: {}", e);
        (axum::http::StatusCode::NOT_FOUND, "게시판을 찾을 수 없습니다".to_string())
    })?;

    let mut context = Context::new();
    context.insert("current_user", &auth.user);
    context.insert("active_page", "boards");
    context.insert("board", &board);
    context.insert("problem_id", &query.problem_id);

    let html = state.tera.render("post_form.html", &context)
        .map_err(|e| {
            eprintln!("Template render error in new_post_form: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Template error: {}", e))
        })?;

    Ok(Html(html).into_response())
}

// 게시글 작성 처리
pub async fn create_post(
    State(state): State<AppState>,
    Path(board_id): Path<i64>,
    auth: AuthSession<Backend>,
    Form(form): Form<CreatePostForm>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let user_id = auth.user.as_ref()
        .map(|u| u.id)
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "로그인이 필요합니다".to_string(),
        ))?;

    // 공지사항 게시판은 관리자만 작성 가능
    let board: (String,) = sqlx::query_as("SELECT board_type FROM boards WHERE id = ?")
        .bind(board_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시판을 찾을 수 없습니다".to_string()))?;

    let is_admin = auth.user.as_ref().map(|u| u.is_admin()).unwrap_or(false);
    if board.0 == "announcement" && !is_admin {
        return Err((axum::http::StatusCode::FORBIDDEN, "권한이 없습니다".to_string()));
    }

    let result = sqlx::query(
        "INSERT INTO posts (board_id, user_id, title, content, problem_id, contest_id)
         VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(board_id)
    .bind(user_id)
    .bind(&form.title)
    .bind(&form.content)
    .bind(form.problem_id)
    .bind(form.contest_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let post_id = result.last_insert_rowid();

    Ok(Redirect::to(&format!("/boards/{}/posts/{}", board_id, post_id)))
}

// 게시글 상세보기
pub async fn post_detail(
    State(state): State<AppState>,
    Path((board_id, post_id)): Path<(i64, i64)>,
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    // 조회수 증가
    sqlx::query("UPDATE posts SET view_count = view_count + 1 WHERE id = ?")
        .bind(post_id)
        .execute(&state.db_pool)
        .await
        .ok();

    // 게시판 정보
    let board = sqlx::query_as::<_, Board>("SELECT * FROM boards WHERE id = ?")
        .bind(board_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시판을 찾을 수 없습니다".to_string()))?;

    // 게시글 정보
    let post = sqlx::query_as::<_, Post>(
        "SELECT p.*, u.username
         FROM posts p
         JOIN users u ON p.user_id = u.id
         WHERE p.id = ? AND p.board_id = ?"
    )
    .bind(post_id)
    .bind(board_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시글을 찾을 수 없습니다".to_string()))?;

    // 댓글 목록 (통계 포함)
    let comments = sqlx::query_as::<_, CommentWithStats>(
        "SELECT c.*, u.username,
                COUNT(DISTINCT cl.id) as like_count
         FROM comments c
         JOIN users u ON c.user_id = u.id
         LEFT JOIN comment_likes cl ON c.id = cl.comment_id
         WHERE c.post_id = ?
         GROUP BY c.id
         ORDER BY c.parent_comment_id IS NULL DESC, c.created_at ASC"
    )
    .bind(post_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // 좋아요 수
    let like_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM post_likes WHERE post_id = ?"
    )
    .bind(post_id)
    .fetch_one(&state.db_pool)
    .await
    .unwrap_or((0,));

    // 현재 사용자가 좋아요를 눌렀는지
    let user_liked = if let Some(user) = &auth.user {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM post_likes WHERE post_id = ? AND user_id = ?"
        )
        .bind(post_id)
        .bind(user.id)
        .fetch_one(&state.db_pool)
        .await
        .unwrap_or(0) > 0
    } else {
        false
    };

    let is_author = auth.user.as_ref().map(|u| u.id == post.user_id).unwrap_or(false);

    let mut context = Context::new();
    context.insert("current_user", &auth.user);
    context.insert("active_page", "boards");
    let user_id = auth.user.as_ref().map(|u| u.id);
    context.insert("user_id", &user_id);
    context.insert("board", &board);
    context.insert("post", &post);
    context.insert("comments", &comments);
    context.insert("like_count", &like_count.0);
    context.insert("user_liked", &user_liked);
    context.insert("is_author", &is_author);

    let html = state.tera.render("post_detail.html", &context)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Html(html))
}

// 댓글 작성
pub async fn create_comment(
    State(state): State<AppState>,
    Path((board_id, post_id)): Path<(i64, i64)>,
    auth: AuthSession<Backend>,
    Form(form): Form<CreateCommentForm>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let user_id = auth.user.as_ref()
        .map(|u| u.id)
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "로그인이 필요합니다".to_string(),
        ))?;

    // 게시글이 잠겨있는지 확인
    let is_locked: (bool,) = sqlx::query_as(
        "SELECT is_locked FROM posts WHERE id = ?"
    )
    .bind(post_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시글을 찾을 수 없습니다".to_string()))?;

    if is_locked.0 {
        return Err((axum::http::StatusCode::FORBIDDEN, "잠긴 게시글입니다".to_string()));
    }

    sqlx::query(
        "INSERT INTO comments (post_id, user_id, parent_comment_id, content)
         VALUES (?, ?, ?, ?)"
    )
    .bind(post_id)
    .bind(user_id)
    .bind(form.parent_comment_id)
    .bind(&form.content)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Redirect::to(&format!("/boards/{}/posts/{}", board_id, post_id)))
}

// 게시글 좋아요 토글
pub async fn toggle_post_like(
    State(state): State<AppState>,
    Path((board_id, post_id)): Path<(i64, i64)>,
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let user_id = auth.user.as_ref()
        .map(|u| u.id)
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "로그인이 필요합니다".to_string(),
        ))?;

    // 이미 좋아요를 눌렀는지 확인
    let exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM post_likes WHERE post_id = ? AND user_id = ?"
    )
    .bind(post_id)
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if exists.0 > 0 {
        // 좋아요 취소
        sqlx::query("DELETE FROM post_likes WHERE post_id = ? AND user_id = ?")
            .bind(post_id)
            .bind(user_id)
            .execute(&state.db_pool)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        // 좋아요 추가
        sqlx::query("INSERT INTO post_likes (post_id, user_id) VALUES (?, ?)")
            .bind(post_id)
            .bind(user_id)
            .execute(&state.db_pool)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Redirect::to(&format!("/boards/{}/posts/{}", board_id, post_id)))
}

// 게시글 삭제
pub async fn delete_post(
    State(state): State<AppState>,
    Path((board_id, post_id)): Path<(i64, i64)>,
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let user_id = auth.user.as_ref()
        .map(|u| u.id)
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "로그인이 필요합니다".to_string(),
        ))?;

    // 게시글 작성자 확인
    let post: (i64,) = sqlx::query_as("SELECT user_id FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시글을 찾을 수 없습니다".to_string()))?;

    // 작성자 본인이거나 관리자만 삭제 가능
    let is_admin = auth.user.as_ref().map(|u| u.is_admin()).unwrap_or(false);
    if post.0 != user_id && !is_admin {
        return Err((axum::http::StatusCode::FORBIDDEN, "권한이 없습니다".to_string()));
    }

    sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(post_id)
        .execute(&state.db_pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Redirect::to(&format!("/boards/{}", board_id)))
}

// 게시글 수정 폼
pub async fn edit_post_form(
    State(state): State<AppState>,
    Path((board_id, post_id)): Path<(i64, i64)>,
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let user_id = auth.user.as_ref()
        .map(|u| u.id)
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "로그인이 필요합니다".to_string(),
        ))?;

    let board = sqlx::query_as::<_, Board>("SELECT * FROM boards WHERE id = ?")
        .bind(board_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시판을 찾을 수 없습니다".to_string()))?;

    let post = sqlx::query_as::<_, Post>(
        "SELECT p.*, u.username FROM posts p JOIN users u ON p.user_id = u.id WHERE p.id = ?"
    )
    .bind(post_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시글을 찾을 수 없습니다".to_string()))?;

    let is_admin = auth.user.as_ref().map(|u| u.is_admin()).unwrap_or(false);
    if post.user_id != user_id && !is_admin {
        return Err((axum::http::StatusCode::FORBIDDEN, "권한이 없습니다".to_string()));
    }

    let mut context = Context::new();
    context.insert("current_user", &auth.user);
    context.insert("active_page", "boards");
    context.insert("board", &board);
    context.insert("post", &post);
    context.insert("is_edit", &true);

    let html = state.tera.render("post_form.html", &context)
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Html(html))
}

// 게시글 수정 처리
pub async fn update_post(
    State(state): State<AppState>,
    Path((board_id, post_id)): Path<(i64, i64)>,
    auth: AuthSession<Backend>,
    Form(form): Form<UpdatePostForm>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let user_id = auth.user.as_ref()
        .map(|u| u.id)
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "로그인이 필요합니다".to_string(),
        ))?;

    let post: (i64,) = sqlx::query_as("SELECT user_id FROM posts WHERE id = ?")
        .bind(post_id)
        .fetch_one(&state.db_pool)
        .await
        .map_err(|_| (axum::http::StatusCode::NOT_FOUND, "게시글을 찾을 수 없습니다".to_string()))?;

    let is_admin = auth.user.as_ref().map(|u| u.is_admin()).unwrap_or(false);
    if post.0 != user_id && !is_admin {
        return Err((axum::http::StatusCode::FORBIDDEN, "권한이 없습니다".to_string()));
    }

    sqlx::query(
        "UPDATE posts SET title = ?, content = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(&form.title)
    .bind(&form.content)
    .bind(post_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Redirect::to(&format!("/boards/{}/posts/{}", board_id, post_id)))
}

// 댓글 좋아요 토글
pub async fn toggle_comment_like(
    State(state): State<AppState>,
    Path((board_id, post_id, comment_id)): Path<(i64, i64, i64)>,
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, (axum::http::StatusCode, String)> {
    let user_id = auth.user.as_ref()
        .map(|u| u.id)
        .ok_or((
            axum::http::StatusCode::UNAUTHORIZED,
            "로그인이 필요합니다".to_string(),
        ))?;

    // 이미 좋아요를 눌렀는지 확인
    let exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM comment_likes WHERE comment_id = ? AND user_id = ?"
    )
    .bind(comment_id)
    .bind(user_id)
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if exists.0 > 0 {
        // 좋아요 취소
        sqlx::query("DELETE FROM comment_likes WHERE comment_id = ? AND user_id = ?")
            .bind(comment_id)
            .bind(user_id)
            .execute(&state.db_pool)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    } else {
        // 좋아요 추가
        sqlx::query("INSERT INTO comment_likes (comment_id, user_id) VALUES (?, ?)")
            .bind(comment_id)
            .bind(user_id)
            .execute(&state.db_pool)
            .await
            .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    }

    Ok(Redirect::to(&format!("/boards/{}/posts/{}", board_id, post_id)))
}
