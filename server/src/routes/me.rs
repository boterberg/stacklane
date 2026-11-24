use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Profile {
    id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    company_name: Option<String>,
    is_business: bool,
    country: String,
    theme: String,
    locale: String,
    timezone: String,
}

#[derive(Deserialize)]
pub struct SettingsRequest {
    theme: Option<String>,
    locale: Option<String>,
    timezone: Option<String>,
}

pub async fn load_profile(state: &AppState, user_id: Uuid) -> AppResult<Profile> {
    sqlx::query_as(
        "SELECT id, email::text AS email, first_name, last_name, company_name, is_business, country::text AS country, theme, locale, timezone
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthenticated)
}

pub async fn show(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Profile>> {
    Ok(Json(load_profile(&state, user.id).await?))
}

pub async fn update_settings(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<SettingsRequest>,
) -> AppResult<Json<Profile>> {
    if let Some(theme) = input.theme.as_deref()
        && !matches!(theme, "light" | "dark" | "system")
    {
        return Err(AppError::field("theme", "Kies licht, donker of systeem."));
    }
    if let Some(locale) = input.locale.as_deref()
        && !matches!(locale, "nl" | "en")
    {
        return Err(AppError::field(
            "locale",
            "Deze taal wordt niet ondersteund.",
        ));
    }
    sqlx::query(
        "UPDATE users SET theme = COALESCE($2, theme), locale = COALESCE($3, locale), timezone = COALESCE($4, timezone), updated_at = now()
         WHERE id = $1",
    )
    .bind(user.id)
    .bind(input.theme)
    .bind(input.locale)
    .bind(input.timezone)
    .execute(&state.db)
    .await?;
    Ok(Json(load_profile(&state, user.id).await?))
}
