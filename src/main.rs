use std::net::SocketAddr;
use std::str::FromStr;
use axum_login::AuthManagerLayerBuilder;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tera::Tera;
use time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::SqliteStore;
use dotenvy::dotenv;
use crate::auth::Backend;

mod router;
mod auth;
mod judge;
mod error;
mod models;
mod handlers;
mod middleware;

#[derive(Clone, axum::extract::FromRef)]
pub struct AppState {
    pub tera: Tera,
    pub db_pool: SqlitePool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // .env 파일에서 환경 변수 로드
    dotenv().ok();
    println!("✅ Environment variables loaded.");

    // 데이터베이스 연결 설정
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let connect_options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true);

    let db_pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;
    println!("✅ Database connected successfully.");

    // 애플리케이션 DB 마이그레이션 실행
    sqlx::migrate!().run(&db_pool).await?;
    println!("✅ Application migrations complete.");

    // 세션 저장소 설정 및 마이그레이션
    let session_store = SqliteStore::new(db_pool.clone());
    session_store.migrate().await?;
    println!("✅ Session table migrations complete.");

    // 세션 레이어 생성
    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    // 인증 백엔드 및 레이어 설정
    let auth_backend = Backend::new(db_pool.clone());
    let auth_layer = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    // Tera 템플릿 엔진 설정
    let tera = Tera::new("templates/**/*")?;

    // 애플리케이션 상태(State) 생성
    let app_state = AppState { tera, db_pool };

    // 라우터 빌드
    let app = router::create_router().with_state(app_state).layer(auth_layer);

    // 서버 실행
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("✅ Server listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
