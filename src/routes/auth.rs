use std::collections::BTreeMap;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{self, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::routes::me::{Profile, load_profile};
use crate::state::AppState;

const MIN_PASSWORD_LENGTH: usize = 10;

#[derive(Deserialize)]
pub struct RegisterRequest {
    name: String,
    email: String,
    password: String,
    country: String,
    #[serde(default)]
    is_business: bool,
    company_name: Option<String>,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
    #[serde(default)]
    remember: bool,
}

#[derive(Deserialize)]
pub struct ForgotPasswordRequest {
    email: String,
}

fn user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn split_name(name: &str) -> (String, String) {
    let trimmed = name.trim();
    match trimmed.split_once(' ') {
        Some((first, last)) => (first.to_owned(), last.trim().to_owned()),
        None => (trimmed.to_owned(), String::new()),
    }
}

fn looks_like_email(email: &str) -> bool {
    let mut parts = email.split('@');
    matches!((parts.next(), parts.next(), parts.next()), (Some(local), Some(domain), None) if !local.is_empty() && domain.contains('.'))
}

pub async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<RegisterRequest>,
) -> AppResult<(CookieJar, Json<Profile>)> {
    let email = input.email.trim().to_lowercase();
    let mut errors = BTreeMap::new();
    if input.name.trim().is_empty() {
        errors.insert("name", "Vul je naam in.".to_owned());
    }
    if !looks_like_email(&email) {
        errors.insert("email", "Vul een geldig e-mailadres in.".to_owned());
    }
    if input.password.chars().count() < MIN_PASSWORD_LENGTH {
        errors.insert(
            "password",
            format!("Je wachtwoord moet minstens {MIN_PASSWORD_LENGTH} tekens lang zijn."),
        );
    }
    if input.country.len() != 2 {
        errors.insert("country", "Kies een land.".to_owned());
    }
    if input.is_business
        && input
            .company_name
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
    {
        errors.insert("company_name", "Vul je bedrijfsnaam in.".to_owned());
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1)")
        .bind(&email)
        .fetch_one(&state.db)
        .await?;
    if taken {
        return Err(AppError::field(
            "email",
            "Er bestaat al een account met dit e-mailadres.",
        ));
    }

    let (first_name, last_name) = split_name(&input.name);
    let user_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, first_name, last_name, country, is_business, company_name)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(user_id)
    .bind(&email)
    .bind(auth::hash_password(&input.password)?)
    .bind(first_name)
    .bind(last_name)
    .bind(input.country.to_uppercase())
    .bind(input.is_business)
    .bind(input.company_name.filter(|_| input.is_business))
    .execute(&state.db)
    .await?;

    let (token, max_age) =
        auth::create_session(&state, user_id, false, user_agent(&headers)).await?;
    let profile = load_profile(&state, user_id).await?;
    Ok((
        jar.add(auth::session_cookie(&state, token, max_age)),
        Json(profile),
    ))
}

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<Profile>)> {
    let email = input.email.trim().to_lowercase();
    let account: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;

    let user_id = match account {
        Some((id, hash)) if auth::verify_password(&input.password, &hash) => id,
        _ => {
            return Err(AppError::field(
                "email",
                "Deze gegevens kloppen niet met onze administratie.",
            ));
        }
    };

    let (token, max_age) =
        auth::create_session(&state, user_id, input.remember, user_agent(&headers)).await?;
    let profile = load_profile(&state, user_id).await?;
    Ok((
        jar.add(auth::session_cookie(&state, token, max_age)),
        Json(profile),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
    user: CurrentUser,
) -> AppResult<(CookieJar, StatusCode)> {
    auth::revoke_session(&state, &user.token_hash).await?;
    Ok((
        jar.add(auth::clear_session_cookie()),
        StatusCode::NO_CONTENT,
    ))
}

pub async fn forgot_password(Json(input): Json<ForgotPasswordRequest>) -> AppResult<StatusCode> {
    if !looks_like_email(input.email.trim()) {
        return Err(AppError::field("email", "Vul een geldig e-mailadres in."));
    }
    Ok(StatusCode::ACCEPTED)
}
