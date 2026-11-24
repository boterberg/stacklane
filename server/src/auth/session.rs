use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

pub const SESSION_COOKIE: &str = "stacklane_session";

const REMEMBERED_LIFETIME: Duration = Duration::days(30);
const DEFAULT_LIFETIME: Duration = Duration::hours(12);

pub struct CurrentUser {
    pub id: Uuid,
    pub token_hash: String,
}

fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

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
    sqlx::query("INSERT INTO sessions (token_hash, user_id, user_agent, expires_at) VALUES ($1, $2, $3, $4)")
        .bind(hash_token(&token))
        .bind(user_id)
        .bind(user_agent)
        .bind(OffsetDateTime::now_utc() + lifetime)
        .execute(&state.db)
        .await?;
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
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|cookie| cookie.value().to_owned())
            .ok_or(AppError::Unauthenticated)?;
        let token_hash = hash_token(&token);
        let user_id: Option<Uuid> = sqlx::query_scalar(
            "UPDATE sessions SET last_seen_at = now() WHERE token_hash = $1 AND expires_at > now() RETURNING user_id",
        )
        .bind(&token_hash)
        .fetch_optional(&state.db)
        .await?;
        let id = user_id.ok_or(AppError::Unauthenticated)?;
        Ok(Self { id, token_hash })
    }
}
