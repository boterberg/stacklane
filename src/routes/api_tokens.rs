use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::auth::{CurrentUser, hash_token, tokens};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

const MAX_TOKENS: i64 = 25;

#[derive(Serialize, sqlx::FromRow)]
pub struct ApiToken {
    id: Uuid,
    name: String,
    prefix: String,
    scopes: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    last_used_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
}

#[derive(Serialize)]
pub struct TokenList {
    tokens: Vec<ApiToken>,
    areas: Vec<&'static str>,
}

#[derive(Deserialize)]
pub struct NewToken {
    name: String,
    scopes: Vec<String>,
    expires_in_days: Option<i64>,
}

#[derive(Serialize)]
pub struct CreatedToken {
    #[serde(flatten)]
    token: ApiToken,
    secret: String,
}

fn browser_only(user: &CurrentUser) -> AppResult<()> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn is_admin(state: &AppState, user: &CurrentUser) -> AppResult<bool> {
    Ok(
        sqlx::query_scalar("SELECT role = 'admin' FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?,
    )
}

pub async fn index(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<TokenList>> {
    browser_only(&user)?;
    let admin = is_admin(&state, &user).await?;
    let tokens = sqlx::query_as(
        "SELECT id, name, prefix, scopes, created_at, last_used_at, expires_at
         FROM api_tokens WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    let areas = tokens::AREAS
        .iter()
        .filter(|area| admin || !area.admin_only)
        .map(|area| area.key)
        .collect();
    Ok(Json(TokenList { tokens, areas }))
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<NewToken>,
) -> AppResult<(StatusCode, Json<CreatedToken>)> {
    browser_only(&user)?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return Err(AppError::field(
            "name",
            "Geef de token een naam van hoogstens 80 tekens.",
        ));
    }
    let admin = is_admin(&state, &user).await?;
    let mut scopes: Vec<String> = input.scopes;
    scopes.sort();
    scopes.dedup();
    if scopes.is_empty() {
        return Err(AppError::field("scopes", "Kies minstens één bevoegdheid."));
    }
    if scopes
        .iter()
        .any(|scope| !tokens::valid_scope(scope, admin))
    {
        return Err(AppError::field("scopes", "Deze bevoegdheid bestaat niet."));
    }
    let expires_at = match input.expires_in_days {
        None => None,
        Some(days @ 1..=365) => Some(OffsetDateTime::now_utc() + Duration::days(days)),
        Some(_) => {
            return Err(AppError::field(
                "expires_in_days",
                "Kies een geldigheid tussen 1 en 365 dagen.",
            ));
        }
    };
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM api_tokens WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
    if count >= MAX_TOKENS {
        return Err(AppError::field(
            "name",
            "Je hebt al 25 tokens. Verwijder er eerst een.",
        ));
    }

    let secret = tokens::generate();
    let prefix: String = secret.chars().take(tokens::PREFIX.len() + 6).collect();
    let token = sqlx::query_as(
        "INSERT INTO api_tokens (id, user_id, name, token_hash, prefix, scopes, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, name, prefix, scopes, created_at, last_used_at, expires_at",
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(name)
    .bind(hash_token(&secret))
    .bind(prefix)
    .bind(&scopes)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(CreatedToken { token, secret })))
}

pub async fn destroy(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    browser_only(&user)?;
    let deleted = sqlx::query("DELETE FROM api_tokens WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}
