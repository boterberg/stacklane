use std::collections::BTreeMap;

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum_extra::extract::CookieJar;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{self, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::i18n::Locale;
use crate::limits::{self, ClientIp};

use crate::mail::Email;
use crate::routes::me::{Profile, load_profile};
use crate::routes::two_factor;
use crate::state::AppState;

pub const MIN_PASSWORD_LENGTH: usize = 10;

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
    code: Option<String>,
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

pub(super) fn looks_like_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    let local_ok = !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(c));
    let labels: Vec<&str> = domain.split('.').collect();
    let domain_ok = labels.len() >= 2
        && domain.len() <= 253
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        });
    email.len() <= 254 && local_ok && domain_ok
}

pub async fn register(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<RegisterRequest>,
) -> AppResult<(CookieJar, Json<Profile>)> {
    state.limits.check(&limits::REGISTER, &ip)?;
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
    if input.name.chars().count() > 100 {
        errors.insert("name", "Gebruik hoogstens 100 tekens.".to_owned());
    }
    if input
        .company_name
        .as_deref()
        .is_some_and(|name| name.chars().count() > 100)
    {
        errors.insert("company_name", "Gebruik hoogstens 100 tekens.".to_owned());
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
    .bind(auth::hash(&input.password).await?)
    .bind(first_name)
    .bind(last_name)
    .bind(input.country.to_uppercase())
    .bind(input.is_business)
    .bind(input.company_name.filter(|_| input.is_business))
    .execute(&state.db)
    .await?;
    super::email_verification::send(&state, user_id, &email).await?;

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
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    headers: HeaderMap,
    Json(input): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<Profile>)> {
    let email = input.email.trim().to_lowercase();
    state.limits.check(&limits::LOGIN, &ip)?;
    state
        .limits
        .check(&limits::LOGIN_ACCOUNT, &format!("{ip}|{email}"))?;
    let account: Option<(Uuid, String, Option<String>)> =
        sqlx::query_as("SELECT id, password_hash, totp_secret FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;

    let matches = auth::verify(
        &input.password,
        account.as_ref().map(|(_, hash, _)| hash.as_str()),
    )
    .await?;
    let user_id = match account {
        Some((id, hash, totp_secret)) if matches => {
            if let Some(sealed) = totp_secret {
                two_factor::check_login_code(&state, id, &sealed, input.code.as_deref()).await?;
            }
            if auth::needs_rehash(&hash) {
                sqlx::query("UPDATE users SET password_hash = $2 WHERE id = $1")
                    .bind(id)
                    .bind(auth::hash(&input.password).await?)
                    .execute(&state.db)
                    .await?;
            }
            id
        }
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

pub async fn forgot_password(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<ForgotPasswordRequest>,
) -> AppResult<StatusCode> {
    state.limits.check(&limits::RECOVERY, &ip)?;
    let email = input.email.trim().to_lowercase();
    if !looks_like_email(&email) {
        return Err(AppError::field("email", "Vul een geldig e-mailadres in."));
    }
    let user: Option<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, first_name, locale FROM users WHERE email = $1")
            .bind(&email)
            .fetch_optional(&state.db)
            .await?;
    let recent: bool = match &user {
        Some((user_id, _, _)) => {
            sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM password_resets
                                WHERE user_id = $1 AND used_at IS NULL AND created_at > now() - interval '2 minutes')",
            )
            .bind(user_id)
            .fetch_one(&state.db)
            .await?
        }
        None => false,
    };
    if let Some((user_id, first_name, locale)) = user.filter(|_| !recent) {
        let token = hex::encode(rand::random::<[u8; 32]>());
        sqlx::query("DELETE FROM password_resets WHERE user_id = $1 AND used_at IS NULL")
            .bind(user_id)
            .execute(&state.db)
            .await?;
        sqlx::query(
            "INSERT INTO password_resets (token_hash, user_id, expires_at) VALUES ($1, $2, now() + interval '1 hour')",
        )
        .bind(auth::hash_token(&token))
        .bind(user_id)
        .execute(&state.db)
        .await?;
        let link = format!("{}/reset-password?token={token}", state.config.public_url);
        let english = Locale::parse(&locale) == Locale::En;
        let greeting = match (english, first_name.is_empty()) {
            (true, true) => "Hello".to_owned(),
            (true, false) => format!("Hello {first_name}"),
            (false, true) => "Hallo".to_owned(),
            (false, false) => format!("Hallo {first_name}"),
        };
        let (subject, body) = if english {
            (
                "Reset your password",
                format!(
                    "{greeting},\n\nYou asked for a new password for your account. Use this link to set it:\n\n{link}\n\nThe link works for one hour. If you did not ask for this, you can ignore this email.\n"
                ),
            )
        } else {
            (
                "Je wachtwoord opnieuw instellen",
                format!(
                    "{greeting},\n\nJe hebt gevraagd om een nieuw wachtwoord voor je account. Via deze link stel je het in:\n\n{link}\n\nDe link werkt een uur. Heb je dit niet zelf gevraagd, dan kun je deze e-mail negeren.\n"
                ),
            )
        };
        if let Err(error) = state
            .mailer
            .send(Email {
                to: &email,
                subject,
                body,
            })
            .await
        {
            tracing::error!(?error, "sending password reset mail failed");
        }
    }
    Ok(StatusCode::ACCEPTED)
}

#[derive(Deserialize)]
pub struct ResetPasswordRequest {
    token: String,
    password: String,
}

pub async fn reset_password(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<ResetPasswordRequest>,
) -> AppResult<StatusCode> {
    state.limits.check(&limits::VERIFY, &ip)?;
    if input.password.chars().count() < MIN_PASSWORD_LENGTH {
        return Err(AppError::field(
            "password",
            format!("Je wachtwoord moet minstens {MIN_PASSWORD_LENGTH} tekens lang zijn."),
        ));
    }
    let mut transaction = state.db.begin().await?;
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE password_resets SET used_at = now()
         WHERE token_hash = $1 AND used_at IS NULL AND expires_at > now()
         RETURNING user_id",
    )
    .bind(auth::hash_token(input.token.trim()))
    .fetch_optional(&mut *transaction)
    .await?;
    let user_id = user_id.ok_or_else(|| {
        AppError::field(
            "token",
            "Deze link is verlopen of al gebruikt. Vraag een nieuwe aan.",
        )
    })?;
    sqlx::query(
        "UPDATE users SET password_hash = $2, email_verified_at = COALESCE(email_verified_at, now()),
                updated_at = now()
         WHERE id = $1",
    )
    .bind(user_id)
    .bind(auth::hash(&input.password).await?)
    .execute(&mut *transaction)
    .await?;
    for table in ["sessions", "api_tokens", "password_resets"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE user_id = $1"
        )))
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod email_tests {
    use super::looks_like_email;

    #[test]
    fn accepts_only_plain_addresses() {
        for good in [
            "jan@example.nl",
            "j.de-vries+shop@mail.example.co.uk",
            "a_b@x-y.io",
        ] {
            assert!(looks_like_email(good), "{good}");
        }
        for bad in [
            "Jan <other@example.nl>",
            "jan@example.nl, other@example.nl",
            "jan@localhost",
            "jan@@example.nl",
            ".jan@example.nl",
            "jan..x@example.nl",
            "jan@-example.nl",
            "jan @example.nl",
            "jan@exa mple.nl",
            "\"jan\"@example.nl",
        ] {
            assert!(!looks_like_email(bad), "{bad}");
        }
    }
}
