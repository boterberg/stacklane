pub mod mollie;
pub mod stripe;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime, Time};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::i18n::Locale;
use crate::settings;
use crate::state::AppState;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stored {
    pub mollie_enabled: bool,
    pub mollie_key: Option<String>,
    pub stripe_enabled: bool,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub bank_transfer_enabled: bool,
}

#[derive(Serialize)]
pub struct ProviderStatus {
    pub mollie: bool,
    pub stripe: bool,
    pub bank_transfer: bool,
}

pub struct Providers {
    pub mollie: Option<mollie::Client>,
    pub stripe: Option<stripe::Client>,
    pub stripe_webhook_secret: Option<String>,
    pub bank_transfer: bool,
}

pub fn reveal(state: &AppState, sealed: Option<&String>) -> Option<String> {
    let secrets = state.config.secrets.as_ref()?;
    let bytes = hex::decode(sealed?).ok()?;
    String::from_utf8(secrets.open(&bytes).ok()?).ok()
}

pub fn conceal(state: &AppState, secret: &str) -> AppResult<String> {
    let secrets = state.config.secrets.as_ref().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("SECRET_KEY ontbreekt in de configuratie"))
    })?;
    Ok(hex::encode(secrets.seal(secret.as_bytes())?))
}

pub async fn providers(state: &AppState) -> AppResult<Providers> {
    let stored: Stored = settings::get(&state.db, "payments").await?;
    Ok(Providers {
        mollie: stored
            .mollie_enabled
            .then(|| reveal(state, stored.mollie_key.as_ref()))
            .flatten()
            .map(mollie::Client::new),
        stripe: stored
            .stripe_enabled
            .then(|| reveal(state, stored.stripe_secret_key.as_ref()))
            .flatten()
            .map(stripe::Client::new),
        stripe_webhook_secret: reveal(state, stored.stripe_webhook_secret.as_ref()),
        bank_transfer: stored.bank_transfer_enabled,
    })
}

impl Providers {
    pub fn status(&self) -> ProviderStatus {
        ProviderStatus {
            mollie: self.mollie.is_some(),
            stripe: self.stripe.is_some(),
            bank_transfer: self.bank_transfer,
        }
    }
}

#[derive(sqlx::FromRow)]
struct Payer {
    user_id: Uuid,
    email: String,
    name: String,
    locale: String,
}

async fn payer(state: &AppState, user_id: Uuid) -> AppResult<Payer> {
    Ok(sqlx::query_as(
        "SELECT id AS user_id, email::text AS email,
                COALESCE(NULLIF(trim(first_name || ' ' || last_name), ''), email::text) AS name, locale
         FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await?)
}

async fn customer_ref(
    state: &AppState,
    provider: &str,
    payer: &Payer,
    providers: &Providers,
) -> AppResult<String> {
    if let Some(existing) = sqlx::query_scalar::<_, String>(
        "SELECT customer_ref FROM payment_customers WHERE user_id = $1 AND provider = $2",
    )
    .bind(payer.user_id)
    .bind(provider)
    .fetch_optional(&state.db)
    .await?
    {
        return Ok(existing);
    }
    let reference = payer.user_id.to_string();
    let created = match provider {
        "mollie" => {
            providers
                .mollie
                .as_ref()
                .context("Mollie staat uit")?
                .create_customer(&payer.name, &payer.email, &reference)
                .await
        }
        _ => {
            providers
                .stripe
                .as_ref()
                .context("Stripe staat uit")?
                .create_customer(&payer.name, &payer.email, &reference)
                .await
        }
    }
    .map_err(AppError::Internal)?;
    sqlx::query("INSERT INTO payment_customers (user_id, provider, customer_ref) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING")
        .bind(payer.user_id)
        .bind(provider)
        .bind(&created)
        .execute(&state.db)
        .await?;
    Ok(created)
}

fn description(locale: Locale, invoice_id: i64) -> String {
    match locale {
        Locale::Nl => format!("Factuur {invoice_id}"),
        Locale::En => format!("Invoice {invoice_id}"),
    }
}

pub async fn start_checkout(
    state: &AppState,
    user_id: Uuid,
    invoice_id: i64,
    provider: &str,
    save_mandate: bool,
) -> AppResult<String> {
    let total: i32 = sqlx::query_scalar(
        "SELECT total_cents FROM invoices WHERE id = $1 AND user_id = $2 AND status = 'open'",
    )
    .bind(invoice_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let providers = providers(state).await?;
    let payer = payer(state, user_id).await?;
    let locale = Locale::parse(&payer.locale);
    let payment_id = Uuid::new_v4();
    let return_url = format!(
        "{}/billing/invoices/{invoice_id}?payment={payment_id}",
        state.config.public_url
    );
    let text = description(locale, invoice_id);

    let (external_id, checkout_url) = match provider {
        "mollie" => {
            let client = providers.mollie.as_ref().ok_or_else(|| {
                AppError::field("provider", "Deze betaalmethode is niet beschikbaar.")
            })?;
            let customer = if save_mandate {
                Some(customer_ref(state, "mollie", &payer, &providers).await?)
            } else {
                None
            };
            let payment = client
                .create_payment(&mollie::NewPayment {
                    amount_cents: i64::from(total),
                    description: &text,
                    redirect_url: &return_url,
                    webhook_url: &format!("{}/api/webhooks/mollie", state.config.public_url),
                    payment_id: &payment_id.to_string(),
                    sequence: match customer.as_deref() {
                        Some(customer) => mollie::Sequence::First { customer },
                        None => mollie::Sequence::OneOff,
                    },
                })
                .await
                .map_err(AppError::Internal)?;
            let url = payment
                .checkout_url()
                .context("Mollie gaf geen betaallink terug")?;
            (payment.id, url)
        }
        "stripe" => {
            let client = providers.stripe.as_ref().ok_or_else(|| {
                AppError::field("provider", "Deze betaalmethode is niet beschikbaar.")
            })?;
            let customer = customer_ref(state, "stripe", &payer, &providers).await?;
            client
                .create_checkout(&stripe::Checkout {
                    customer: &customer,
                    amount_cents: i64::from(total),
                    description: &text,
                    success_url: &return_url,
                    cancel_url: &format!("{return_url}&cancelled=1"),
                    payment_id: &payment_id.to_string(),
                    save_card: save_mandate,
                })
                .await
                .map_err(AppError::Internal)?
        }
        _ => {
            return Err(AppError::field(
                "provider",
                "Deze betaalmethode is niet beschikbaar.",
            ));
        }
    };

    sqlx::query(
        "INSERT INTO payments (id, invoice_id, provider, external_id, status, amount_cents, checkout_url, save_mandate)
         VALUES ($1, $2, $3, $4, 'open', $5, $6, $7)",
    )
    .bind(payment_id)
    .bind(invoice_id)
    .bind(provider)
    .bind(&external_id)
    .bind(total)
    .bind(&checkout_url)
    .bind(save_mandate)
    .execute(&state.db)
    .await?;
    Ok(checkout_url)
}

pub async fn record_mandate(
    state: &AppState,
    payment_id: Uuid,
    user_id: Uuid,
    provider: &str,
    customer: &str,
    mandate: &str,
    description: &str,
) -> AppResult<()> {
    let mut transaction = state.db.begin().await?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM payments WHERE id = $1 FOR UPDATE")
            .bind(payment_id)
            .fetch_optional(&mut *transaction)
            .await?;
    if status.is_none_or(|status| status == "paid") {
        return Ok(());
    }
    let (id, revoked): (Uuid, bool) = sqlx::query_as(
        "INSERT INTO payment_mandates (id, user_id, provider, customer_ref, mandate_ref, description)
         VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (provider, mandate_ref) DO UPDATE SET description = EXCLUDED.description
         RETURNING id, revoked_at IS NOT NULL",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(provider)
    .bind(customer)
    .bind(mandate)
    .bind(description)
    .fetch_one(&mut *transaction)
    .await?;
    if !revoked {
        sqlx::query(
            "UPDATE subscriptions s SET mandate_id = $2, payment_mode = 'automatic'
             WHERE s.user_id = $1 AND s.status IN ('active', 'pending', 'suspended') AND s.cancelled_at IS NULL
               AND EXISTS (SELECT 1 FROM invoice_lines l JOIN payments p ON p.invoice_id = l.invoice_id
                           WHERE l.subscription_id = s.id AND p.id = $3 AND p.save_mandate)",
        )
        .bind(user_id)
        .bind(id)
        .bind(payment_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn update_payment(
    state: &AppState,
    payment_id: Uuid,
    status: &str,
    method: Option<&str>,
) -> AppResult<()> {
    let previous: Option<(String, i64)> =
        sqlx::query_as("SELECT status, invoice_id FROM payments WHERE id = $1")
            .bind(payment_id)
            .fetch_optional(&state.db)
            .await?;
    let Some((previous, invoice_id)) = previous else {
        return Ok(());
    };
    if previous == "paid" {
        return Ok(());
    }
    sqlx::query(
        "UPDATE payments SET status = $2, method = COALESCE($3, method),
                paid_at = CASE WHEN $2 = 'paid' THEN now() ELSE paid_at END
         WHERE id = $1",
    )
    .bind(payment_id)
    .bind(status)
    .bind(method)
    .execute(&state.db)
    .await?;
    if status == "paid" {
        mark_invoice_paid(state, invoice_id).await?;
    }
    Ok(())
}

fn add_months(moment: OffsetDateTime, months: i32) -> OffsetDateTime {
    let date = moment.date();
    let total = date.year() * 12 + i32::from(u8::from(date.month())) - 1 + months;
    let (year, month) = (total.div_euclid(12), (total.rem_euclid(12) + 1) as u8);
    let month = time::Month::try_from(month).unwrap_or(time::Month::January);
    let day = date.day().min(month.length(year));
    time::Date::from_calendar_date(year, month, day)
        .map(|date| date.with_time(moment.time()).assume_offset(moment.offset()))
        .unwrap_or(moment)
}

pub fn interval_months(interval: &str) -> i32 {
    match interval {
        "quarterly" => 3,
        "half_yearly" => 6,
        "yearly" => 12,
        _ => 1,
    }
}

pub fn next_expiry(current: OffsetDateTime, interval: &str) -> OffsetDateTime {
    add_months(current, interval_months(interval))
}

async fn flag_late_payment(state: &AppState, invoice_id: i64) -> AppResult<()> {
    let cancelled: bool =
        sqlx::query_scalar("SELECT status = 'cancelled' FROM invoices WHERE id = $1")
            .bind(invoice_id)
            .fetch_optional(&state.db)
            .await?
            .unwrap_or(false);
    if !cancelled {
        return Ok(());
    }
    tracing::warn!(invoice_id, "a payment came in for a cancelled invoice");
    let admins: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM users WHERE role = 'admin' AND deleted_at IS NULL")
            .fetch_all(&state.db)
            .await?;
    for admin in admins {
        crate::renewals::notify(
            state,
            admin,
            (
                format!("Betaling op geannuleerde factuur {invoice_id}"),
                format!("Er is betaald voor factuur {invoice_id}, maar die factuur was al geannuleerd. Controleer de betaling en betaal hem zo nodig terug."),
            ),
            (
                format!("Payment on cancelled invoice {invoice_id}"),
                format!("Invoice {invoice_id} was paid after it had been cancelled. Check the payment and refund it if needed."),
            ),
            &format!("/admin/invoices/{invoice_id}"),
        )
        .await
        .ok();
    }
    Ok(())
}

pub async fn mark_invoice_paid(state: &AppState, invoice_id: i64) -> AppResult<()> {
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE invoices SET status = 'paid', paid_at = now() WHERE id = $1 AND status = 'open' RETURNING user_id",
    )
    .bind(invoice_id)
    .fetch_optional(&state.db)
    .await?;
    let Some(user_id) = user_id else {
        return flag_late_payment(state, invoice_id).await;
    };

    let covered: Vec<(Uuid, Option<time::Date>)> = sqlx::query_as(
        "SELECT subscription_id, max(period_end) FROM invoice_lines
         WHERE invoice_id = $1 AND subscription_id IS NOT NULL GROUP BY subscription_id",
    )
    .bind(invoice_id)
    .fetch_all(&state.db)
    .await?;
    for (subscription_id, period_end) in covered {
        let (status, expires_at, interval): (String, OffsetDateTime, String) = sqlx::query_as(
            "SELECT status, expires_at, billing_interval FROM subscriptions WHERE id = $1",
        )
        .bind(subscription_id)
        .fetch_one(&state.db)
        .await?;
        let paid_until = period_end
            .and_then(|end| end.next_day())
            .map(|day| day.with_time(Time::MIDNIGHT).assume_utc())
            .unwrap_or_else(|| next_expiry(expires_at, &interval));
        sqlx::query(
            "UPDATE subscriptions SET expires_at = GREATEST(expires_at, $2), status = 'active', suspended_for_payment = FALSE
             WHERE id = $1",
        )
        .bind(subscription_id)
        .bind(paid_until)
        .execute(&state.db)
        .await?;
        sqlx::query(
            "UPDATE invoices v SET status = 'cancelled'
             WHERE v.status = 'open' AND v.id <> $3
               AND EXISTS (SELECT 1 FROM invoice_lines l WHERE l.invoice_id = v.id)
               AND NOT EXISTS (
                   SELECT 1 FROM invoice_lines l WHERE l.invoice_id = v.id
                     AND (l.subscription_id IS DISTINCT FROM $1 OR l.period_end IS NULL OR l.period_end >= $2::date))",
        )
        .bind(subscription_id)
        .bind(paid_until)
        .bind(invoice_id)
        .execute(&state.db)
        .await?;
        let services: Vec<(Uuid, String)> =
            sqlx::query_as(
                "SELECT id, status FROM services
                 WHERE subscription_id = $1
                   AND (status = 'pending' OR (status = 'suspended' AND suspension_reason = 'payment'))",
            )
                .bind(subscription_id)
                .fetch_all(&state.db)
                .await?;
        for (service_id, service_status) in services {
            if service_status == "pending" || status == "pending" {
                crate::services::provision_in_background(state.clone(), service_id);
            } else if let Err(error) =
                crate::services::change_state_because(state, service_id, "unsuspend", "payment")
                    .await
            {
                tracing::warn!(%service_id, ?error, "unsuspending after payment failed");
            }
        }
        if status != "pending" {
            let domains: Vec<Uuid> = sqlx::query_scalar(
                "SELECT id FROM services WHERE subscription_id = $1 AND kind = 'domain' AND status IN ('active', 'suspended')",
            )
            .bind(subscription_id)
            .fetch_all(&state.db)
            .await?;
            for service_id in domains {
                crate::services::renew_domain_in_background(state.clone(), service_id);
            }
        }
    }

    let locale: String = sqlx::query_scalar("SELECT locale FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&state.db)
        .await?;
    let (title, body) = match Locale::parse(&locale) {
        Locale::Nl => (
            "Betaling ontvangen".to_owned(),
            format!("We hebben de betaling voor factuur {invoice_id} ontvangen. Bedankt!"),
        ),
        Locale::En => (
            "Payment received".to_owned(),
            format!("We received the payment for invoice {invoice_id}. Thank you!"),
        ),
    };
    sqlx::query(
        "INSERT INTO notifications (id, user_id, title, body, link) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(title)
    .bind(body)
    .bind(format!("/billing/invoices/{invoice_id}"))
    .execute(&state.db)
    .await?;
    Ok(())
}

pub fn renewal_start(expires_at: OffsetDateTime) -> time::Date {
    expires_at.date()
}

pub fn renewal_end(expires_at: OffsetDateTime, interval: &str) -> time::Date {
    next_expiry(expires_at, interval).date() - Duration::days(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn adds_months_without_overflowing_short_months() {
        assert_eq!(
            add_months(datetime!(2026-01-31 10:00 UTC), 1),
            datetime!(2026-02-28 10:00 UTC)
        );
        assert_eq!(
            add_months(datetime!(2026-11-15 00:00 UTC), 3),
            datetime!(2027-02-15 00:00 UTC)
        );
        assert_eq!(
            next_expiry(datetime!(2026-05-25 00:00 UTC), "yearly"),
            datetime!(2027-05-25 00:00 UTC)
        );
    }

    #[test]
    fn renewal_periods_end_the_day_before_the_next_expiry() {
        let expires = datetime!(2026-05-25 00:00 UTC);
        assert_eq!(renewal_start(expires).to_string(), "2026-05-25");
        assert_eq!(renewal_end(expires, "monthly").to_string(), "2026-06-24");
    }
}
