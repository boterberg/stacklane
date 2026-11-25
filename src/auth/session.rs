use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use super::tokens;
use crate::error::AppError;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "stacklane_session";

const REMEMBERED_LIFETIME: Duration = Duration::days(30);
const DEFAULT_LIFETIME: Duration = Duration::hours(12);

pub struct CurrentUser {
    pub id: Uuid,
    pub token_hash: String,
    pub via_api_token: bool,
}

pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

const USER_AGENT_CHARS: usize = 256;
const MAX_SESSIONS: i64 = 50;

pub async fn create_session(
    state: &AppState,
    user_id: Uuid,
    remember: bool,
    user_agent: Option<String>,
) -> Result<(String, Option<Duration>), AppError> {
    let token = hex::encode(rand::random::<[u8; 32]>());
    let lifetime = if remember {
        REMEMBERED_LIFETIME
    } else {
        DEFAULT_LIFETIME
    };
    let user_agent =
        user_agent.map(|agent| agent.chars().take(USER_AGENT_CHARS).collect::<String>());
    let mut transaction = state.db.begin().await?;
    sqlx::query("INSERT INTO sessions (token_hash, user_id, user_agent, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(user_agent)
        .bind(OffsetDateTime::now_utc() + lifetime)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "DELETE FROM sessions WHERE user_id = $1 AND token_hash IN (
             SELECT token_hash FROM sessions WHERE user_id = $1
             ORDER BY created_at DESC OFFSET $2
         )",
    )
    .bind(user_id)
    .bind(MAX_SESSIONS)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok((token, remember.then_some(lifetime)))
}

pub async fn revoke_session(state: &AppState, token_hash: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = $1")
        .bind(token_hash)
        .execute(&state.db)
        .await?;
    Ok(())
}

pub fn session_cookie(
    state: &AppState,
    token: String,
    max_age: Option<Duration>,
) -> Cookie<'static> {
    let mut cookie = Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .build();
    if let Some(max_age) = max_age {
        cookie.set_max_age(max_age);
    }
    cookie
}

pub fn clear_session_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .max_age(Duration::ZERO)
        .build()
}

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let bearer = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim);
        if let Some(token) = bearer {
            return from_api_token(parts, state, token).await;
        }
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|cookie| cookie.value().to_owned())
            .ok_or(AppError::Unauthenticated)?;
        let token_hash = hash_token(&token);
        let user_id: Option<Uuid> = sqlx::query_scalar(
            "UPDATE sessions s SET last_seen_at = now() FROM users u
             WHERE s.token_hash = $1 AND s.expires_at > now() AND u.id = s.user_id AND u.deleted_at IS NULL
             RETURNING s.user_id",
        )
        .bind(&token_hash)
        .fetch_optional(&state.db)
        .await?;
        let id = user_id.ok_or(AppError::Unauthenticated)?;
        Ok(Self {
            id,
            token_hash,
            via_api_token: false,
        })
    }
}

async fn from_api_token(
    parts: &Parts,
    state: &AppState,
    token: &str,
) -> Result<CurrentUser, AppError> {
    if !token.starts_with(tokens::PREFIX) {
        return Err(AppError::Unauthenticated);
    }
    let token_hash = hash_token(token);
    let found: Option<(Uuid, Vec<String>)> = sqlx::query_as(
        "UPDATE api_tokens t SET last_used_at = now() FROM users u
         WHERE t.token_hash = $1 AND (t.expires_at IS NULL OR t.expires_at > now())
           AND u.id = t.user_id AND u.deleted_at IS NULL
         RETURNING t.user_id, t.scopes",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?;
    let (id, scopes) = found.ok_or(AppError::Unauthenticated)?;
    if !tokens::allows(&scopes, &parts.method, parts.uri.path()) {
        return Err(AppError::Forbidden);
    }
    Ok(CurrentUser {
        id,
        token_hash,
        via_api_token: true,
    })
}

pub struct CurrentAdmin {
    pub id: Uuid,
    pub via_api_token: bool,
}

impl CurrentAdmin {
    pub fn browser_only(&self) -> Result<(), AppError> {
        if self.via_api_token {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }
}

pub const PERMISSIONS: [&str; 4] = ["support", "billing", "customers", "services"];

pub trait Permission {
    const KEY: &'static str;
}

pub mod permission {
    pub struct Support;
    pub struct Billing;
    pub struct Customers;
    pub struct Services;

    impl super::Permission for Support {
        const KEY: &'static str = "support";
    }

    impl super::Permission for Billing {
        const KEY: &'static str = "billing";
    }

    impl super::Permission for Customers {
        const KEY: &'static str = "customers";
    }

    impl super::Permission for Services {
        const KEY: &'static str = "services";
    }
}

pub struct Staff<P> {
    pub id: Uuid,
    permission: std::marker::PhantomData<P>,
}

pub async fn staff_with(db: &sqlx::PgPool, key: &str) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        "SELECT id FROM users WHERE deleted_at IS NULL AND (role = 'admin' OR (role = 'staff' AND $1 = ANY(staff_permissions)))",
    )
    .bind(key)
    .fetch_all(db)
    .await
}

pub fn role_allows(role: &str, permissions: &[String], key: &str) -> bool {
    role == "admin" || (role == "staff" && permissions.iter().any(|granted| granted == key))
}

impl<P: Permission> FromRequestParts<AppState> for Staff<P> {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let (role, permissions): (String, Vec<String>) =
            sqlx::query_as("SELECT role, staff_permissions FROM users WHERE id = $1")
                .bind(user.id)
                .fetch_one(&state.db)
                .await?;
        if !role_allows(&role, &permissions, P::KEY) {
            return Err(AppError::Forbidden);
        }
        Ok(Self {
            id: user.id,
            permission: std::marker::PhantomData,
        })
    }
}

impl FromRequestParts<AppState> for CurrentAdmin {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let is_admin: bool = sqlx::query_scalar("SELECT role = 'admin' FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
        if !is_admin {
            return Err(AppError::Forbidden);
        }
        Ok(Self {
            id: user.id,
            via_api_token: user.via_api_token,
        })
    }
}
