use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::{self, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::i18n::Locale;
use crate::limits::{self, ClientIp};
use crate::mail::Email;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct VerifyRequest {
    token: String,
}

pub fn greeting(first_name: &str, english: bool) -> String {
    match (english, first_name.is_empty()) {
        (true, true) => "Hello".to_owned(),
        (true, false) => format!("Hello {first_name}"),
        (false, true) => "Hallo".to_owned(),
        (false, false) => format!("Hallo {first_name}"),
    }
}

pub async fn refuse_if_just_mailed(state: &AppState, user_id: Uuid, email: &str) -> AppResult<()> {
    let recent: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM email_verifications
                        WHERE lower(email::text) = lower($2) AND user_id <> $1
                          AND created_at > now() - interval '10 minutes')",
    )
    .bind(user_id)
    .bind(email)
    .fetch_one(&state.db)
    .await?;
    if recent {
        return Err(AppError::field(
            "email",
            "Er is net al een bevestiging naar dit adres gestuurd. Probeer het over een paar minuten opnieuw.",
        ));
    }
    Ok(())
}

pub async fn send(state: &AppState, user_id: Uuid, email: &str) -> AppResult<()> {
    let (first_name, locale, current): (String, String, String) =
        sqlx::query_as("SELECT first_name, locale, email::text FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;
    let token = hex::encode(rand::random::<[u8; 32]>());
    let mut transaction = state.db.begin().await?;
    sqlx::query("DELETE FROM email_verifications WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "INSERT INTO email_verifications (token_hash, user_id, email, expires_at)
         VALUES ($1, $2, $3, now() + interval '24 hours')",
    )
    .bind(auth::hash_token(&token))
    .bind(user_id)
    .bind(email)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let english = Locale::parse(&locale) == Locale::En;
    let greeting = greeting(&first_name, english);
    let link = format!("{}/verify-email?token={token}", state.config.public_url);
    let change = !current.eq_ignore_ascii_case(email);
    let (subject, body) = match (english, change) {
        (true, false) => (
            "Confirm your email address",
            format!(
                "{greeting},\n\nPlease confirm that this is your email address:\n\n{link}\n\nThe link works for 24 hours. If you did not create an account, you can ignore this email.\n"
            ),
        ),
        (true, true) => (
            "Confirm your new email address",
            format!(
                "{greeting},\n\nYou asked to use this address for your account. Confirm it with this link:\n\n{link}\n\nThe link works for 24 hours. Until then you keep logging in with your old address.\n"
            ),
        ),
        (false, false) => (
            "Bevestig je e-mailadres",
            format!(
                "{greeting},\n\nBevestig dat dit jouw e-mailadres is:\n\n{link}\n\nDe link werkt 24 uur. Heb je geen account aangemaakt, dan kun je deze e-mail negeren.\n"
            ),
        ),
        (false, true) => (
            "Bevestig je nieuwe e-mailadres",
            format!(
                "{greeting},\n\nJe wilt dit adres voortaan voor je account gebruiken. Bevestig het met deze link:\n\n{link}\n\nDe link werkt 24 uur. Tot dan log je in met je oude adres.\n"
            ),
        ),
    };
    if let Err(error) = state
        .mailer
        .send(Email {
            to: email,
            subject,
            body,
        })
        .await
    {
        tracing::error!(?error, "sending a verification mail failed");
    }
    if change {
        let (subject, body) = if english {
            (
                "Your email address is about to change",
                format!(
                    "{greeting},\n\nSomeone asked to change the email address of your account to {email}. The change only happens after that address is confirmed.\n\nIf this was not you, change your password and contact us.\n"
                ),
            )
        } else {
            (
                "Je e-mailadres wordt gewijzigd",
                format!(
                    "{greeting},\n\nEr is gevraagd om het e-mailadres van je account te wijzigen in {email}. Dat gebeurt pas als dat adres is bevestigd.\n\nWas jij dit niet? Wijzig dan je wachtwoord en neem contact met ons op.\n"
                ),
            )
        };
        if let Err(error) = state
            .mailer
            .send(Email {
                to: &current,
                subject,
                body,
            })
            .await
        {
            tracing::error!(?error, "sending an email change notice failed");
        }
    }
    Ok(())
}

pub async fn verify(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    Json(input): Json<VerifyRequest>,
) -> AppResult<StatusCode> {
    state.limits.check(&limits::VERIFY, &ip)?;
    let expired = || {
        AppError::field(
            "token",
            "Deze link is verlopen of al gebruikt. Vraag een nieuwe aan.",
        )
    };
    let mut transaction = state.db.begin().await?;
    let found: Option<(Uuid, String)> = sqlx::query_as(
        "DELETE FROM email_verifications WHERE token_hash = $1 AND expires_at > now()
         RETURNING user_id, email::text",
    )
    .bind(auth::hash_token(input.token.trim()))
    .fetch_optional(&mut *transaction)
    .await?;
    let (user_id, email) = found.ok_or_else(expired)?;
    let taken: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1 AND id <> $2)")
            .bind(&email)
            .bind(user_id)
            .fetch_one(&mut *transaction)
            .await?;
    if taken {
        return Err(AppError::field(
            "token",
            "Er bestaat al een account met dit e-mailadres.",
        ));
    }
    let previous: String = sqlx::query_scalar("SELECT email::text FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&mut *transaction)
        .await?;
    sqlx::query(
        "UPDATE users SET email = $2, email_verified_at = now(), updated_at = now() WHERE id = $1",
    )
    .bind(user_id)
    .bind(&email)
    .execute(&mut *transaction)
    .await?;
    if !previous.eq_ignore_ascii_case(&email) {
        sqlx::query("DELETE FROM password_resets WHERE user_id = $1")
            .bind(user_id)
            .execute(&mut *transaction)
            .await?;
    }
    sqlx::query("DELETE FROM email_verifications WHERE user_id = $1")
        .bind(user_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn resend(State(state): State<AppState>, user: CurrentUser) -> AppResult<StatusCode> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    let recent: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM email_verifications
                        WHERE user_id = $1 AND created_at > now() - interval '1 minute')",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    if recent {
        return Err(AppError::field(
            "email",
            "We hebben net een e-mail gestuurd. Probeer het over een minuut opnieuw.",
        ));
    }
    let (email, verified): (String, bool) = sqlx::query_as(
        "SELECT COALESCE((SELECT v.email::text FROM email_verifications v WHERE v.user_id = u.id
                          ORDER BY v.created_at DESC LIMIT 1), u.email::text),
                u.email_verified_at IS NOT NULL
         FROM users u WHERE u.id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    let pending: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM email_verifications WHERE user_id = $1)")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    if verified && !pending {
        return Ok(StatusCode::NO_CONTENT);
    }
    refuse_if_just_mailed(&state, user.id, &email).await?;
    send(&state, user.id, &email).await?;
    Ok(StatusCode::ACCEPTED)
}

pub async fn cancel(State(state): State<AppState>, user: CurrentUser) -> AppResult<StatusCode> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    sqlx::query(
        "DELETE FROM email_verifications v USING users u
         WHERE v.user_id = $1 AND u.id = v.user_id AND v.email <> u.email",
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
