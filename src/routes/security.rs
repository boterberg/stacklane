use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;

use crate::auth::{self, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

use super::auth::MIN_PASSWORD_LENGTH;

#[derive(Deserialize)]
pub struct PasswordChange {
    current_password: String,
    password: String,
}

#[derive(Deserialize)]
pub struct Confirm {
    password: String,
}

async fn check_password(
    state: &AppState,
    user: &CurrentUser,
    password: &str,
    field: &'static str,
) -> AppResult<()> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
    if !auth::verify_password(password, &hash) {
        return Err(AppError::field(field, "Dit wachtwoord klopt niet."));
    }
    Ok(())
}

pub async fn change_password(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<PasswordChange>,
) -> AppResult<StatusCode> {
    if state.config.demo_mode {
        return Err(AppError::field(
            "password",
            "In de demo kun je het wachtwoord niet wijzigen.",
        ));
    }
    check_password(&state, &user, &input.current_password, "current_password").await?;
    if input.password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err(AppError::field(
            "password",
            format!("Je wachtwoord moet minstens {MIN_PASSWORD_LENGTH} tekens lang zijn."),
        ));
    }
    sqlx::query("UPDATE users SET password_hash = $2, updated_at = now() WHERE id = $1")
        .bind(user.id)
        .bind(auth::hash_password(&input.password)?)
        .execute(&state.db)
        .await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2")
        .bind(user.id)
        .bind(&user.token_hash)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn logout_other_sessions(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<Confirm>,
) -> AppResult<StatusCode> {
    check_password(&state, &user, &input.password, "password").await?;
    sqlx::query("DELETE FROM sessions WHERE user_id = $1 AND token_hash <> $2")
        .bind(user.id)
        .bind(&user.token_hash)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
