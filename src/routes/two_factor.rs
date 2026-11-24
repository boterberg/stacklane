use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::{CurrentUser, hash_token, totp};
use crate::error::{AppError, AppResult};
use crate::payments::{conceal, reveal};
use crate::settings::{self, Company};
use crate::state::AppState;

use super::security::check_password;

const RECOVERY_CODES: usize = 8;

#[derive(Serialize)]
pub struct Status {
    enabled: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    enabled_at: Option<OffsetDateTime>,
    recovery_codes_left: i64,
}

#[derive(Serialize)]
pub struct Setup {
    secret: String,
    uri: String,
    qr_svg: String,
}

#[derive(Deserialize)]
pub struct Code {
    code: String,
}

#[derive(Deserialize)]
pub struct Password {
    password: String,
}

#[derive(Serialize)]
pub struct RecoveryCodes {
    recovery_codes: Vec<String>,
}

fn browser_only(user: &CurrentUser) -> AppResult<()> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn status(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Status>> {
    browser_only(&user)?;
    let (enabled_at, recovery_codes_left): (Option<OffsetDateTime>, i64) = sqlx::query_as(
        "SELECT u.two_factor_enabled_at,
                (SELECT count(*) FROM recovery_codes r WHERE r.user_id = u.id AND r.used_at IS NULL)
         FROM users u WHERE u.id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    Ok(Json(Status {
        enabled: enabled_at.is_some(),
        enabled_at,
        recovery_codes_left,
    }))
}

pub async fn setup(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Setup>> {
    browser_only(&user)?;
    if state.config.demo_mode {
        return Err(AppError::field(
            "code",
            "In de demo kun je 2FA niet aanzetten.",
        ));
    }
    let (email, enabled): (String, bool) = sqlx::query_as(
        "SELECT email::text, two_factor_enabled_at IS NOT NULL FROM users WHERE id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    if enabled {
        return Err(AppError::field("code", "2FA staat al aan."));
    }
    let company: Company = settings::get(&state.db, "company").await?;
    let issuer = if company.name.trim().is_empty() {
        "Stacklane".to_owned()
    } else {
        company.name
    };
    let secret = totp::new_secret();
    sqlx::query("UPDATE users SET totp_pending = $2 WHERE id = $1")
        .bind(user.id)
        .bind(conceal(&state, &hex::encode(&secret))?)
        .execute(&state.db)
        .await?;
    let uri = totp::provisioning_uri(&secret, &issuer, &email);
    Ok(Json(Setup {
        secret: totp::base32(&secret),
        qr_svg: totp::qr_svg(&uri),
        uri,
    }))
}

async fn replace_recovery_codes(state: &AppState, user_id: Uuid) -> AppResult<Vec<String>> {
    let codes: Vec<String> = (0..RECOVERY_CODES).map(|_| totp::recovery_code()).collect();
    let mut transaction = state.db.begin().await?;
    sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    for code in &codes {
        sqlx::query("INSERT INTO recovery_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(hash_token(code))
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok(codes)
}

pub async fn enable(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<Code>,
) -> AppResult<Json<RecoveryCodes>> {
    browser_only(&user)?;
    let pending: Option<String> = sqlx::query_scalar(
        "SELECT totp_pending FROM users WHERE id = $1 AND two_factor_enabled_at IS NULL",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .flatten();
    let secret = reveal(&state, pending.as_ref())
        .and_then(|value| hex::decode(value).ok())
        .ok_or_else(|| AppError::field("code", "Begin opnieuw met instellen."))?;
    let step = totp::verify(
        &secret,
        &input.code,
        OffsetDateTime::now_utc().unix_timestamp(),
        None,
    )
    .ok_or_else(|| {
        AppError::field(
            "code",
            "Deze code klopt niet. Kijk of de tijd op je telefoon goed staat.",
        )
    })?;
    sqlx::query(
        "UPDATE users SET totp_secret = totp_pending, totp_pending = NULL, totp_last_step = $2, two_factor_enabled_at = now()
         WHERE id = $1",
    )
    .bind(user.id)
    .bind(step)
    .execute(&state.db)
    .await?;
    let recovery_codes = replace_recovery_codes(&state, user.id).await?;
    Ok(Json(RecoveryCodes { recovery_codes }))
}

pub async fn disable(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<Password>,
) -> AppResult<StatusCode> {
    check_password(&state, &user, &input.password, "password").await?;
    sqlx::query(
        "UPDATE users SET totp_secret = NULL, totp_pending = NULL, totp_last_step = NULL, two_factor_enabled_at = NULL
         WHERE id = $1",
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;
    sqlx::query("DELETE FROM recovery_codes WHERE user_id = $1")
        .bind(user.id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn new_recovery_codes(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<Password>,
) -> AppResult<Json<RecoveryCodes>> {
    check_password(&state, &user, &input.password, "password").await?;
    let enabled: bool =
        sqlx::query_scalar("SELECT two_factor_enabled_at IS NOT NULL FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    if !enabled {
        return Err(AppError::field("password", "Zet eerst 2FA aan."));
    }
    let recovery_codes = replace_recovery_codes(&state, user.id).await?;
    Ok(Json(RecoveryCodes { recovery_codes }))
}

pub async fn check_login_code(
    state: &AppState,
    user_id: Uuid,
    sealed: &str,
    code: Option<&str>,
) -> AppResult<()> {
    let code = code
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .ok_or_else(|| AppError::field("code", "Vul de code uit je authenticator-app in."))?;
    let wrong = || AppError::field("code", "Deze code klopt niet.");
    if code.contains('-') {
        let used = sqlx::query(
            "UPDATE recovery_codes SET used_at = now() WHERE user_id = $1 AND code_hash = $2 AND used_at IS NULL",
        )
        .bind(user_id)
        .bind(hash_token(&code.to_lowercase()))
        .execute(&state.db)
        .await?
        .rows_affected();
        return if used == 1 { Ok(()) } else { Err(wrong()) };
    }
    let secret = reveal(state, Some(&sealed.to_owned()))
        .and_then(|value| hex::decode(value).ok())
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "cannot open the 2FA secret, was SECRET_KEY changed?"
            ))
        })?;
    let last_step: Option<i64> =
        sqlx::query_scalar("SELECT totp_last_step FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;
    let step = totp::verify(
        &secret,
        code,
        OffsetDateTime::now_utc().unix_timestamp(),
        last_step,
    )
    .ok_or_else(wrong)?;
    sqlx::query("UPDATE users SET totp_last_step = $2 WHERE id = $1")
        .bind(user_id)
        .bind(step)
        .execute(&state.db)
        .await?;
    Ok(())
}
