use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::{CurrentAdmin, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::payments::{self, ProviderStatus, Stored, mollie, stripe};
use crate::settings::{self, Company};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Mandate {
    id: Uuid,
    provider: String,
    description: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct PaymentOptions {
    providers: ProviderStatus,
    iban: Option<String>,
    account_holder: Option<String>,
    mandates: Vec<Mandate>,
}

pub async fn options(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<PaymentOptions>> {
    let providers = payments::providers(&state).await?;
    let company: Company = settings::get(&state.db, "company").await?;
    let mandates = sqlx::query_as(
        "SELECT id, provider, description, created_at FROM payment_mandates
         WHERE user_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    let bank = providers.bank_transfer;
    Ok(Json(PaymentOptions {
        providers: providers.status(),
        iban: if bank { company.iban } else { None },
        account_holder: bank.then_some(company.name).filter(|name| !name.is_empty()),
        mandates,
    }))
}

#[derive(Deserialize)]
pub struct PayRequest {
    provider: String,
    #[serde(default)]
    save: bool,
}

#[derive(Serialize)]
pub struct Checkout {
    checkout_url: String,
}

pub async fn pay(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
    Json(input): Json<PayRequest>,
) -> AppResult<Json<Checkout>> {
    if state.config.demo_mode {
        return Err(AppError::field(
            "provider",
            "In de demo kun je niet echt betalen.",
        ));
    }
    state
        .limits
        .check(&crate::limits::CHECKOUT, &user.id.to_string())?;
    let checkout_url =
        payments::start_checkout(&state, user.id, id, &input.provider, input.save).await?;
    Ok(Json(Checkout { checkout_url }))
}

pub async fn revoke_mandate(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let revoked = sqlx::query("UPDATE payment_mandates SET revoked_at = now() WHERE id = $1 AND user_id = $2 AND revoked_at IS NULL")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?
        .rows_affected();
    if revoked == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "UPDATE subscriptions SET mandate_id = NULL, payment_mode = 'manual' WHERE mandate_id = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    sqlx::query(
        "UPDATE users SET default_mandate_id = NULL, default_payment_mode = 'manual'
         WHERE id = $1 AND default_mandate_id = $2",
    )
    .bind(user.id)
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct MollieWebhook {
    id: String,
}

pub async fn mollie_webhook(
    State(state): State<AppState>,
    Form(input): Form<MollieWebhook>,
) -> StatusCode {
    if let Err(error) = handle_mollie(&state, &input.id).await {
        tracing::warn!(payment = %input.id, ?error, "processing a Mollie webhook failed");
    }
    StatusCode::OK
}

async fn handle_mollie(state: &AppState, external_id: &str) -> AppResult<()> {
    let found: Option<(Uuid, bool, Uuid)> = sqlx::query_as(
        "SELECT p.id, p.save_mandate, v.user_id FROM payments p JOIN invoices v ON v.id = p.invoice_id
         WHERE p.provider = 'mollie' AND p.external_id = $1",
    )
    .bind(external_id)
    .fetch_optional(&state.db)
    .await?;
    let providers = payments::providers(state).await?;
    let client = providers.mollie.ok_or(AppError::NotFound)?;
    let (payment_id, save_mandate, user_id, payment) = match found {
        Some((payment_id, save_mandate, user_id)) => {
            let payment = client
                .payment(external_id)
                .await
                .map_err(AppError::Internal)?;
            (payment_id, save_mandate, user_id, payment)
        }
        None => {
            if !looks_like_mollie_payment(external_id) {
                return Ok(());
            }
            state
                .limits
                .check(&crate::limits::UNKNOWN_PAYMENT, "mollie")?;
            let payment = client
                .payment(external_id)
                .await
                .map_err(AppError::Internal)?;
            let Some(local) = payment
                .metadata
                .get("payment_id")
                .and_then(Value::as_str)
                .and_then(|id| id.parse::<Uuid>().ok())
            else {
                return Ok(());
            };
            let claimed: Option<(Uuid, bool, Uuid)> = sqlx::query_as(
                "UPDATE payments p SET external_id = $2 FROM invoices v
                 WHERE p.id = $1 AND v.id = p.invoice_id AND p.provider = 'mollie' AND p.external_id IS NULL
                 RETURNING p.id, p.save_mandate, v.user_id",
            )
            .bind(local)
            .bind(external_id)
            .fetch_optional(&state.db)
            .await?;
            let Some((payment_id, save_mandate, user_id)) = claimed else {
                return Ok(());
            };
            (payment_id, save_mandate, user_id, payment)
        }
    };
    let status = mollie::map_status(&payment.status);
    let method = payment.describe_method();
    if status == "paid"
        && save_mandate
        && let (Some(customer), Some(mandate)) = (&payment.customer_id, &payment.mandate_id)
    {
        payments::record_mandate(
            state, payment_id, user_id, "mollie", customer, mandate, &method,
        )
        .await?;
    }
    payments::update_payment(state, payment_id, status, Some(&method)).await
}

fn looks_like_mollie_payment(id: &str) -> bool {
    id.strip_prefix("tr_").is_some_and(|rest| {
        (1..=32).contains(&rest.len()) && rest.chars().all(|c| c.is_ascii_alphanumeric())
    })
}

pub async fn stripe_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    let providers = match payments::providers(&state).await {
        Ok(providers) => providers,
        Err(_) => return StatusCode::SERVICE_UNAVAILABLE,
    };
    let (Some(secret), Some(signature)) = (
        providers.stripe_webhook_secret.as_deref(),
        headers
            .get("stripe-signature")
            .and_then(|value| value.to_str().ok()),
    ) else {
        return StatusCode::BAD_REQUEST;
    };
    if !stripe::verify_signature(
        &body,
        signature,
        secret,
        OffsetDateTime::now_utc().unix_timestamp(),
    ) {
        return StatusCode::BAD_REQUEST;
    }
    let Ok(event) = serde_json::from_slice::<Value>(&body) else {
        return StatusCode::BAD_REQUEST;
    };
    if let Err(error) = handle_stripe(&state, &providers, &event).await {
        tracing::warn!(?error, "processing a Stripe webhook failed");
    }
    StatusCode::OK
}

fn text<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

async fn handle_stripe(
    state: &AppState,
    providers: &payments::Providers,
    event: &Value,
) -> AppResult<()> {
    let object = event.pointer("/data/object").cloned().unwrap_or_default();
    let payment_id = text(&object, "/client_reference_id")
        .or_else(|| text(&object, "/metadata/payment_id"))
        .and_then(|id| id.parse::<Uuid>().ok());
    let Some(payment_id) = payment_id else {
        return Ok(());
    };
    let found: Option<(bool, Uuid)> = sqlx::query_as(
        "SELECT p.save_mandate, v.user_id FROM payments p JOIN invoices v ON v.id = p.invoice_id
         WHERE p.id = $1 AND p.provider = 'stripe'",
    )
    .bind(payment_id)
    .fetch_optional(&state.db)
    .await?;
    let Some((save_mandate, user_id)) = found else {
        return Ok(());
    };
    let client = providers.stripe.as_ref().ok_or(AppError::NotFound)?;

    match text(event, "/type").unwrap_or_default() {
        "checkout.session.completed" | "checkout.session.async_payment_succeeded" => {
            if text(&object, "/payment_status") != Some("paid") {
                return Ok(());
            }
            let intent_id = text(&object, "/payment_intent").unwrap_or_default();
            let intent = client
                .payment_intent(intent_id)
                .await
                .map_err(AppError::Internal)?;
            let method = intent.get("payment_method").cloned().unwrap_or_default();
            let description = stripe::describe_card(&method);
            if save_mandate
                && let (Some(customer), Some(method_id)) =
                    (text(&object, "/customer"), text(&method, "/id"))
            {
                payments::record_mandate(
                    state,
                    payment_id,
                    user_id,
                    "stripe",
                    customer,
                    method_id,
                    &description,
                )
                .await?;
            }
            payments::update_payment(state, payment_id, "paid", Some(&description)).await
        }
        "checkout.session.expired" => {
            payments::update_payment(state, payment_id, "expired", None).await
        }
        "payment_intent.succeeded" => {
            let intent_id = text(&object, "/id").unwrap_or_default();
            let intent = client
                .payment_intent(intent_id)
                .await
                .map_err(AppError::Internal)?;
            let method = intent.get("payment_method").cloned().unwrap_or_default();
            let description = stripe::describe_card(&method);
            if save_mandate
                && let (Some(customer), Some(method_id)) =
                    (text(&intent, "/customer"), text(&method, "/id"))
            {
                payments::record_mandate(
                    state,
                    payment_id,
                    user_id,
                    "stripe",
                    customer,
                    method_id,
                    &description,
                )
                .await?;
            }
            payments::update_payment(state, payment_id, "paid", Some(&description)).await
        }
        "payment_intent.payment_failed" => {
            payments::update_payment(state, payment_id, "failed", None).await
        }
        _ => Ok(()),
    }
}

#[derive(Serialize)]
pub struct PaymentSettings {
    mollie_enabled: bool,
    mollie_configured: bool,
    stripe_enabled: bool,
    stripe_configured: bool,
    stripe_webhook_configured: bool,
    bank_transfer_enabled: bool,
    webhook_urls: Value,
}

#[derive(Deserialize)]
pub struct PaymentSettingsUpdate {
    mollie_enabled: bool,
    mollie_key: Option<String>,
    stripe_enabled: bool,
    stripe_secret_key: Option<String>,
    stripe_webhook_secret: Option<String>,
    bank_transfer_enabled: bool,
}

fn view(state: &AppState, stored: &Stored) -> PaymentSettings {
    PaymentSettings {
        mollie_enabled: stored.mollie_enabled,
        mollie_configured: stored.mollie_key.is_some(),
        stripe_enabled: stored.stripe_enabled,
        stripe_configured: stored.stripe_secret_key.is_some(),
        stripe_webhook_configured: stored.stripe_webhook_secret.is_some(),
        bank_transfer_enabled: stored.bank_transfer_enabled,
        webhook_urls: serde_json::json!({
            "mollie": format!("{}/api/webhooks/mollie", state.config.public_url),
            "stripe": format!("{}/api/webhooks/stripe", state.config.public_url),
        }),
    }
}

pub async fn admin_settings(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<PaymentSettings>> {
    let stored: Stored = settings::get(&state.db, "payments").await?;
    Ok(Json(view(&state, &stored)))
}

fn fresh(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

pub async fn update_admin_settings(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(input): Json<PaymentSettingsUpdate>,
) -> AppResult<Json<PaymentSettings>> {
    let mut stored: Stored = settings::get(&state.db, "payments").await?;
    if let Some(key) = fresh(input.mollie_key) {
        mollie::Client::new(key.clone())
            .check()
            .await
            .map_err(|error| AppError::field("mollie_key", format!("{error:#}")))?;
        stored.mollie_key = Some(payments::conceal(&state, &key)?);
    }
    if let Some(key) = fresh(input.stripe_secret_key) {
        stripe::Client::new(key.clone())
            .check()
            .await
            .map_err(|error| AppError::field("stripe_secret_key", format!("{error:#}")))?;
        stored.stripe_secret_key = Some(payments::conceal(&state, &key)?);
    }
    if let Some(secret) = fresh(input.stripe_webhook_secret) {
        stored.stripe_webhook_secret = Some(payments::conceal(&state, &secret)?);
    }
    if input.mollie_enabled && stored.mollie_key.is_none() {
        return Err(AppError::field(
            "mollie_key",
            "Vul eerst een API-sleutel van Mollie in.",
        ));
    }
    if input.stripe_enabled
        && (stored.stripe_secret_key.is_none() || stored.stripe_webhook_secret.is_none())
    {
        return Err(AppError::field(
            "stripe_secret_key",
            "Vul eerst de geheime sleutel en het webhookgeheim van Stripe in.",
        ));
    }
    stored.mollie_enabled = input.mollie_enabled;
    stored.stripe_enabled = input.stripe_enabled;
    stored.bank_transfer_enabled = input.bank_transfer_enabled;
    settings::set(&state.db, "payments", &stored).await?;
    Ok(Json(view(&state, &stored)))
}

#[cfg(test)]
mod tests {
    use super::looks_like_mollie_payment;

    #[test]
    fn only_mollie_payment_ids_reach_the_provider() {
        assert!(looks_like_mollie_payment("tr_WDqYK6vllg"));
        for id in [
            "",
            "tr_",
            "cst_abc",
            "tr_../x",
            "tr_a b",
            &format!("tr_{}", "a".repeat(33)),
        ] {
            assert!(!looks_like_mollie_payment(id), "{id}");
        }
    }
}
