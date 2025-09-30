use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Clone, Debug, Deserialize)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug, Default, sqlx::FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    password_hash: String,
}

impl AuthUser for User {
    type Id = i64;
    fn id(&self) -> Self::Id { self.id }
    fn session_auth_hash(&self) -> &[u8] { self.password_hash.as_bytes() }
}

impl User {
    pub fn verify_password(&self, password: &str) -> bool {
        PasswordHash::new(&self.password_hash)
            .and_then(|hash| Argon2::default().verify_password(password.as_bytes(), &hash))
            .is_ok()
    }
}

#[derive(Clone, Debug)]
pub struct Backend {
    db_pool: SqlitePool,
}

impl Backend {
    pub fn new(db_pool: SqlitePool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = sqlx::Error;

    async fn authenticate(&self, creds: Self::Credentials) -> Result<Option<Self::User>, Self::Error> {
        let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = ?")
            .bind(creds.username)
            .fetch_optional(&self.db_pool)
            .await?;

        let Some(user) = user else { return Ok(None) };

        if user.verify_password(&creds.password) { Ok(Some(user)) } else { Ok(None) }
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        sqlx::query_as("SELECT * FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&self.db_pool)
            .await
    }
}

