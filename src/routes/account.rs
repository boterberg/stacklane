use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{self, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::vat;

use super::auth::looks_like_email;
use super::security::check_password;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct AccountDetails {
    first_name: String,
    last_name: String,
    email: String,
    invoice_email: Option<String>,
    phone: Option<String>,
    is_business: bool,
    company_name: Option<String>,
    coc_number: Option<String>,
    tax_id: Option<String>,
    country: String,
    address_line1: Option<String>,
    address_line2: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    #[serde(default, skip_deserializing)]
    tax_id_verified: bool,
    #[serde(default, skip_serializing)]
    #[sqlx(default)]
    current_password: Option<String>,
    #[serde(default, skip_serializing)]
    #[sqlx(default)]
    code: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PaymentDefaults {
    mode: String,
    mandate_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct Password {
    password: String,
}

fn browser_only(user: &CurrentUser) -> AppResult<()> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

fn tidy(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn load(state: &AppState, user_id: Uuid) -> AppResult<AccountDetails> {
    Ok(sqlx::query_as(
        "SELECT first_name, last_name, email::text AS email, invoice_email::text AS invoice_email, phone, is_business,
                company_name, coc_number, tax_id, country::text AS country, address_line1, address_line2, postal_code, city,
                tax_id_verified_at IS NOT NULL AS tax_id_verified
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?)
}

pub async fn show(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<AccountDetails>> {
    Ok(Json(load(&state, user.id).await?))
}

pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<AccountDetails>,
) -> AppResult<Json<AccountDetails>> {
    let first_name = input.first_name.trim();
    if first_name.is_empty() {
        return Err(AppError::field("first_name", "Vul je voornaam in."));
    }
    let email = input.email.trim().to_lowercase();
    let stored: (String, Option<String>) =
        sqlx::query_as("SELECT email::text, invoice_email FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    if email != stored.0.to_lowercase() && !looks_like_email(&email) {
        return Err(AppError::field("email", "Vul een geldig e-mailadres in."));
    }
    let invoice_email = tidy(input.invoice_email).map(|value| value.to_lowercase());
    if invoice_email.as_deref().is_some_and(|value| {
        Some(value) != stored.1.as_deref().map(str::to_lowercase).as_deref()
            && !looks_like_email(value)
    }) {
        return Err(AppError::field(
            "invoice_email",
            "Vul een geldig e-mailadres in.",
        ));
    }
    let country = input.country.trim().to_uppercase();
    if country.len() != 2
        || !country
            .chars()
            .all(|character| character.is_ascii_uppercase())
    {
        return Err(AppError::field("country", "Kies een land."));
    }
    let company_name = tidy(input.company_name);
    for (field, value) in [
        ("first_name", Some(first_name)),
        ("last_name", Some(input.last_name.as_str())),
        ("company_name", company_name.as_deref()),
    ] {
        if value.is_some_and(|value| value.chars().count() > 100) {
            return Err(AppError::field(field, "Gebruik hoogstens 100 tekens."));
        }
    }
    for (field, value, max) in [
        ("phone", input.phone.as_deref(), 32),
        ("coc_number", input.coc_number.as_deref(), 32),
        ("tax_id", input.tax_id.as_deref(), 20),
        ("address_line1", input.address_line1.as_deref(), 200),
        ("address_line2", input.address_line2.as_deref(), 200),
        ("postal_code", input.postal_code.as_deref(), 20),
        ("city", input.city.as_deref(), 100),
    ] {
        if value.is_some_and(|value| value.trim().chars().count() > max) {
            return Err(AppError::field(
                field,
                format!("Gebruik hoogstens {max} tekens."),
            ));
        }
    }
    if input.is_business && company_name.is_none() {
        return Err(AppError::field(
            "company_name",
            "Vul de naam van je bedrijf in.",
        ));
    }
    let (current_email, totp_secret): (String, Option<String>) =
        sqlx::query_as("SELECT email::text, totp_secret FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    let changing = current_email != email;
    if changing {
        super::email_verification::refuse_if_just_mailed(&state, user.id, &email).await?;
        if state.config.demo_mode {
            return Err(AppError::field(
                "email",
                "In de demo kun je het e-mailadres niet wijzigen.",
            ));
        }
        let taken: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE email = $1 AND id <> $2)")
                .bind(&email)
                .bind(user.id)
                .fetch_one(&state.db)
                .await?;
        if taken {
            return Err(AppError::field(
                "email",
                "Er bestaat al een account met dit e-mailadres.",
            ));
        }
        check_password(
            &state,
            &user,
            input.current_password.as_deref().unwrap_or_default(),
            "current_password",
        )
        .await?;
        if let Some(sealed) = totp_secret {
            super::two_factor::check_login_code(&state, user.id, &sealed, input.code.as_deref())
                .await?;
        }
    }
    let tax_id = tidy(input.tax_id)
        .filter(|_| input.is_business)
        .map(|value| value.to_uppercase());
    let previous: (Option<String>, String, bool) = sqlx::query_as(
        "SELECT tax_id, country::text, tax_id_verified_at IS NOT NULL FROM users WHERE id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    let unchanged = previous.0 == tax_id && previous.1 == country;
    let verified = match tax_id
        .as_deref()
        .and_then(|value| vat::split_vat_number(value, &country))
    {
        None if tax_id.is_some() => {
            return Err(AppError::field(
                "tax_id",
                "Vul een btw-nummer uit de EU in, zoals NL123456789B01.",
            ));
        }
        None => false,
        Some(_) if unchanged && previous.2 => true,
        Some((code, number)) => {
            state
                .limits
                .check(&crate::limits::VAT_CHECK, &user.id.to_string())?;
            match vat::check_vies(&code, &number).await {
                Ok(true) => true,
                Ok(false) => {
                    return Err(AppError::field(
                        "tax_id",
                        "Dit btw-nummer is niet geldig volgens VIES.",
                    ));
                }
                Err(error) => {
                    tracing::warn!(error = %format!("{error:#}"), "checking a VAT number failed");
                    false
                }
            }
        }
    };
    sqlx::query(
        "UPDATE users SET first_name = $2, last_name = $3, email = $4, invoice_email = $5, phone = $6, is_business = $7,
                company_name = $8, coc_number = $9, tax_id = $10, country = $11, address_line1 = $12, address_line2 = $13,
                postal_code = $14, city = $15,
                tax_id_verified_at = CASE WHEN $16 THEN COALESCE(tax_id_verified_at, now()) END, updated_at = now()
         WHERE id = $1",
    )
    .bind(user.id)
    .bind(first_name)
    .bind(input.last_name.trim())
    .bind(&current_email)
    .bind(invoice_email)
    .bind(tidy(input.phone))
    .bind(input.is_business)
    .bind(company_name.filter(|_| input.is_business))
    .bind(tidy(input.coc_number).filter(|_| input.is_business))
    .bind(&tax_id)
    .bind(&country)
    .bind(tidy(input.address_line1))
    .bind(tidy(input.address_line2))
    .bind(tidy(input.postal_code).map(|value| value.to_uppercase()))
    .bind(tidy(input.city))
    .bind(verified)
    .execute(&state.db)
    .await?;
    if changing {
        super::email_verification::send(&state, user.id, &email).await?;
    }
    Ok(Json(load(&state, user.id).await?))
}

pub async fn payment_defaults(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<PaymentDefaults>> {
    let (mode, mandate_id): (String, Option<Uuid>) =
        sqlx::query_as("SELECT default_payment_mode, default_mandate_id FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    Ok(Json(PaymentDefaults { mode, mandate_id }))
}

pub async fn update_payment_defaults(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<PaymentDefaults>,
) -> AppResult<Json<PaymentDefaults>> {
    let mandate_id = match input.mode.as_str() {
        "manual" => None,
        "automatic" => {
            let mandate_id = input.mandate_id.ok_or_else(|| {
                AppError::field("mandate_id", "Kies een opgeslagen betaalmethode.")
            })?;
            let valid: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM payment_mandates WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL)",
            )
            .bind(mandate_id)
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
            if !valid {
                return Err(AppError::field(
                    "mandate_id",
                    "Kies een opgeslagen betaalmethode.",
                ));
            }
            Some(mandate_id)
        }
        _ => return Err(AppError::field("mode", "Kies handmatig of automatisch.")),
    };
    sqlx::query(
        "UPDATE users SET default_payment_mode = $2, default_mandate_id = $3 WHERE id = $1",
    )
    .bind(user.id)
    .bind(&input.mode)
    .bind(mandate_id)
    .execute(&state.db)
    .await?;
    sqlx::query(
        "UPDATE subscriptions SET payment_mode = $2, mandate_id = $3
         WHERE user_id = $1 AND cancelled_at IS NULL AND status IN ('active', 'pending', 'suspended')",
    )
    .bind(user.id)
    .bind(&input.mode)
    .bind(mandate_id)
    .execute(&state.db)
    .await?;
    Ok(Json(PaymentDefaults {
        mode: input.mode,
        mandate_id,
    }))
}

pub async fn destroy(
    State(state): State<AppState>,
    jar: CookieJar,
    user: CurrentUser,
    Json(input): Json<Password>,
) -> AppResult<(CookieJar, StatusCode)> {
    browser_only(&user)?;
    if state.config.demo_mode {
        return Err(AppError::field(
            "password",
            "In de demo kun je je account niet verwijderen.",
        ));
    }
    check_password(&state, &user, &input.password, "password").await?;
    let (running, unpaid, services, networks): (bool, bool, bool, bool) = sqlx::query_as(
        "SELECT
            EXISTS (SELECT 1 FROM subscriptions WHERE user_id = $1 AND status IN ('active', 'pending', 'suspended') AND cancelled_at IS NULL),
            EXISTS (SELECT 1 FROM invoices WHERE user_id = $1 AND status = 'open'),
            EXISTS (SELECT 1 FROM services WHERE user_id = $1 AND status NOT IN ('terminated', 'failed')),
            EXISTS (SELECT 1 FROM vps_networks WHERE user_id = $1)",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    if running || unpaid {
        return Err(AppError::field(
            "password",
            "Zeg eerst je abonnementen op en betaal je openstaande facturen.",
        ));
    }
    if services {
        return Err(AppError::field(
            "password",
            "Er draaien nog diensten op je account. Wacht tot ze beëindigd zijn of vraag ons ze te stoppen.",
        ));
    }
    if networks {
        return Err(AppError::field(
            "password",
            "Verwijder eerst je privénetwerken onder VPS, Netwerken.",
        ));
    }

    let mut transaction = state.db.begin().await?;
    sqlx::query(
        "UPDATE users SET email = 'verwijderd-' || id || '@invalid', password_hash = '!', first_name = '', last_name = '',
                company_name = NULL, coc_number = NULL, tax_id = NULL, invoice_email = NULL, phone = NULL,
                address_line1 = NULL, address_line2 = NULL, city = NULL, postal_code = NULL, province = NULL,
                totp_secret = NULL, totp_pending = NULL, two_factor_enabled_at = NULL, default_mandate_id = NULL,
                deleted_at = now(), updated_at = now()
         WHERE id = $1",
    )
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE payment_mandates SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("DELETE FROM vps_members WHERE owner_id = $1 OR user_id = $1")
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
    for table in [
        "sessions",
        "api_tokens",
        "notifications",
        "recovery_codes",
        "password_resets",
        "email_verifications",
        "whois_profiles",
        "ssh_keys",
        "vps_firewalls",
    ] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DELETE FROM {table} WHERE user_id = $1"
        )))
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    tracing::info!(user = %user.id, "account deleted");
    Ok((
        jar.add(auth::clear_session_cookie()),
        StatusCode::NO_CONTENT,
    ))
}
