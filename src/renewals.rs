use std::time::Duration;

use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::AppResult;
use crate::i18n::Locale;
use crate::mail::Email;
use crate::payments::{self, interval_months, mollie, renewal_end, renewal_start, stripe};
use crate::settings::{self, Billing};
use crate::state::AppState;

const RUN_EVERY: Duration = Duration::from_secs(60 * 60);

#[derive(sqlx::FromRow)]
struct Due {
    id: Uuid,
    user_id: Uuid,
    code: String,
    expires_at: OffsetDateTime,
    billing_interval: String,
    payment_mode: String,
    mandate_id: Option<Uuid>,
}

pub fn tax_included(total_cents: i64, rate_basis_points: i32) -> i64 {
    let rate = i64::from(rate_basis_points);
    total_cents - (total_cents * 10_000 + (10_000 + rate) / 2) / (10_000 + rate)
}

async fn notify(
    state: &AppState,
    user_id: Uuid,
    nl: (String, String),
    en: (String, String),
    link: &str,
) -> AppResult<()> {
    let (email, locale): (String, String) =
        sqlx::query_as("SELECT email::text, locale FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await?;
    let (title, body) = match Locale::parse(&locale) {
        Locale::Nl => nl,
        Locale::En => en,
    };
    sqlx::query(
        "INSERT INTO notifications (id, user_id, title, body, link) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(user_id)
    .bind(&title)
    .bind(&body)
    .bind(link)
    .execute(&state.db)
    .await?;
    let message = format!("{body}\n\n{}{link}\n", state.config.public_url);
    if let Err(error) = state
        .mailer
        .send(Email {
            to: &email,
            subject: &title,
            body: message,
        })
        .await
    {
        tracing::warn!(?error, "sending a billing email failed");
    }
    Ok(())
}

async fn create_invoice(
    state: &AppState,
    due: &Due,
    billing: &Billing,
) -> AppResult<Option<(i64, i64)>> {
    let items: Vec<(String, i32)> = sqlx::query_as(
        "SELECT description, monthly_price_cents FROM subscription_items WHERE subscription_id = $1 ORDER BY description",
    )
    .bind(due.id)
    .fetch_all(&state.db)
    .await?;
    let months = i64::from(interval_months(&due.billing_interval));
    let total: i64 = items
        .iter()
        .map(|(_, price)| i64::from(*price) * months)
        .sum();
    if items.is_empty() {
        return Ok(None);
    }
    let start = renewal_start(due.expires_at);
    let end = renewal_end(due.expires_at, &due.billing_interval);
    let today = OffsetDateTime::now_utc().date();
    let due_at = start.max(today);
    let snapshot: Value = sqlx::query_scalar(
        "SELECT jsonb_build_object(
            'name', COALESCE(NULLIF(trim(first_name || ' ' || last_name), ''), email::text),
            'company_name', company_name, 'address_line1', address_line1, 'address_line2', address_line2,
            'postal_code', postal_code, 'city', city, 'country', country::text)
         FROM users WHERE id = $1",
    )
    .bind(due.user_id)
    .fetch_one(&state.db)
    .await?;

    let mut transaction = state.db.begin().await?;
    let invoice_id: i64 = sqlx::query_scalar(
        "INSERT INTO invoices (user_id, status, issued_at, due_at, tax_rate_basis_points, total_cents, tax_cents, billing_snapshot)
         VALUES ($1, 'open', $2, $3, $4, $5, $6, $7) RETURNING id",
    )
    .bind(due.user_id)
    .bind(today)
    .bind(due_at)
    .bind(billing.tax_rate_basis_points)
    .bind(i32::try_from(total).unwrap_or(i32::MAX))
    .bind(i32::try_from(tax_included(total, billing.tax_rate_basis_points)).unwrap_or(0))
    .bind(snapshot)
    .fetch_one(&mut *transaction)
    .await?;
    for (description, price) in &items {
        sqlx::query(
            "INSERT INTO invoice_lines (id, invoice_id, subscription_id, description, period_start, period_end, amount_cents)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(invoice_id)
        .bind(due.id)
        .bind(description)
        .bind(start)
        .bind(end)
        .bind(i32::try_from(i64::from(*price) * months).unwrap_or(i32::MAX))
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(Some((invoice_id, total)))
}

async fn charge(state: &AppState, invoice_id: i64, total: i64, mandate_id: Uuid) -> AppResult<()> {
    let mandate: Option<(String, String, String)> = sqlx::query_as(
        "SELECT provider, customer_ref, mandate_ref FROM payment_mandates WHERE id = $1 AND revoked_at IS NULL",
    )
    .bind(mandate_id)
    .fetch_optional(&state.db)
    .await?;
    let Some((provider, customer, mandate)) = mandate else {
        return Ok(());
    };
    let providers = payments::providers(state).await?;
    let payment_id = Uuid::new_v4();
    let description = format!("Stacklane {invoice_id}");
    sqlx::query(
        "INSERT INTO payments (id, invoice_id, provider, status, amount_cents, recurring) VALUES ($1, $2, $3, 'open', $4, TRUE)",
    )
    .bind(payment_id)
    .bind(invoice_id)
    .bind(&provider)
    .bind(i32::try_from(total).unwrap_or(i32::MAX))
    .execute(&state.db)
    .await?;

    let outcome = match (provider.as_str(), &providers.mollie, &providers.stripe) {
        ("mollie", Some(client), _) => client
            .create_payment(&mollie::NewPayment {
                amount_cents: total,
                description: &description,
                redirect_url: "",
                webhook_url: &format!("{}/api/webhooks/mollie", state.config.public_url),
                payment_id: &payment_id.to_string(),
                sequence: mollie::Sequence::Recurring {
                    customer: &customer,
                    mandate: &mandate,
                },
            })
            .await
            .map(|payment| (payment.id, mollie::map_status(&payment.status))),
        ("stripe", _, Some(client)) => client
            .charge_off_session(
                &customer,
                &mandate,
                total,
                &description,
                &payment_id.to_string(),
            )
            .await
            .map(|intent| {
                (
                    intent
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_owned(),
                    stripe::map_intent_status(
                        intent
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                    ),
                )
            }),
        _ => Err(anyhow::anyhow!(
            "de betaalprovider van deze machtiging staat uit"
        )),
    };
    match outcome {
        Ok((external_id, status)) => {
            sqlx::query("UPDATE payments SET external_id = $2 WHERE id = $1")
                .bind(payment_id)
                .bind(external_id)
                .execute(&state.db)
                .await?;
            payments::update_payment(state, payment_id, status, None).await
        }
        Err(error) => {
            tracing::warn!(invoice_id, ?error, "automatic charge failed");
            payments::update_payment(state, payment_id, "failed", None).await
        }
    }
}

async fn issue_renewals(state: &AppState, billing: &Billing) -> AppResult<usize> {
    let due: Vec<Due> = sqlx::query_as(
        "SELECT s.id, s.user_id, s.code, s.expires_at, s.billing_interval, s.payment_mode, s.mandate_id
         FROM subscriptions s
         WHERE s.status IN ('active', 'suspended') AND s.cancelled_at IS NULL
           AND s.expires_at <= now() + make_interval(days => $1)
           AND NOT EXISTS (
               SELECT 1 FROM invoice_lines l JOIN invoices v ON v.id = l.invoice_id
               WHERE l.subscription_id = s.id AND l.period_start = s.expires_at::date AND v.status IN ('open', 'paid'))",
    )
    .bind(billing.renewal_notice_days)
    .fetch_all(&state.db)
    .await?;
    let mut created = 0;
    for subscription in due {
        let Some((invoice_id, total)) = create_invoice(state, &subscription, billing).await? else {
            continue;
        };
        created += 1;
        let link = format!("/billing/invoices/{invoice_id}");
        match (subscription.payment_mode.as_str(), subscription.mandate_id) {
            ("automatic", Some(mandate_id)) => charge(state, invoice_id, total, mandate_id).await?,
            _ => notify(
                state,
                subscription.user_id,
                ("Nieuwe factuur".into(), format!("Factuur {invoice_id} voor abonnement {} staat klaar. Betaal hem voor de vervaldatum, dan loopt alles gewoon door.", subscription.code)),
                ("New invoice".into(), format!("Invoice {invoice_id} for subscription {} is ready. Pay it before the due date and everything keeps running.", subscription.code)),
                &link,
            )
            .await?,
        }
    }
    Ok(created)
}

async fn suspend_overdue(state: &AppState, billing: &Billing) -> AppResult<usize> {
    let overdue: Vec<(Uuid, Uuid, String)> = sqlx::query_as(
        "SELECT s.id, s.user_id, s.code FROM subscriptions s
         WHERE s.status = 'active' AND s.cancelled_at IS NULL
           AND s.expires_at + make_interval(days => $1) < now()
           AND EXISTS (
               SELECT 1 FROM invoice_lines l JOIN invoices v ON v.id = l.invoice_id
               WHERE l.subscription_id = s.id AND v.status = 'open')",
    )
    .bind(billing.suspend_after_days)
    .fetch_all(&state.db)
    .await?;
    for (subscription_id, user_id, code) in &overdue {
        sqlx::query("UPDATE subscriptions SET status = 'suspended', suspended_for_payment = TRUE WHERE id = $1")
            .bind(subscription_id)
            .execute(&state.db)
            .await?;
        let services: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM services WHERE subscription_id = $1 AND status = 'active'",
        )
        .bind(subscription_id)
        .fetch_all(&state.db)
        .await?;
        for service_id in services {
            if let Err(error) = crate::services::change_state(state, service_id, "suspend").await {
                tracing::warn!(%service_id, ?error, "suspending an unpaid service failed");
            }
        }
        notify(
            state,
            *user_id,
            ("Abonnement opgeschort".into(), format!("Abonnement {code} is opgeschort omdat de factuur niet is betaald. Na betaling staat alles meteen weer aan.")),
            ("Subscription suspended".into(), format!("Subscription {code} has been suspended because the invoice was not paid. Everything comes back as soon as you pay.")),
            "/billing/invoices",
        )
        .await?;
    }
    Ok(overdue.len())
}

async fn end_cancelled(state: &AppState) -> AppResult<usize> {
    let ended: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE subscriptions SET status = 'cancelled'
         WHERE status IN ('active', 'suspended') AND cancelled_at IS NOT NULL AND expires_at < now()
         RETURNING id",
    )
    .fetch_all(&state.db)
    .await?;
    for subscription_id in &ended {
        let services: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM services WHERE subscription_id = $1 AND status = 'active'",
        )
        .bind(subscription_id)
        .fetch_all(&state.db)
        .await?;
        for service_id in services {
            if let Err(error) = crate::services::change_state(state, service_id, "suspend").await {
                tracing::warn!(%service_id, ?error, "suspending an ended service failed");
            }
        }
    }
    Ok(ended.len())
}

pub async fn run_once(state: &AppState) -> AppResult<(usize, usize, usize)> {
    let billing: Billing = settings::get(&state.db, "billing").await?;
    Ok((
        issue_renewals(state, &billing).await?,
        suspend_overdue(state, &billing).await?,
        end_cancelled(state).await?,
    ))
}

pub fn spawn(state: AppState) {
    if state.config.demo_mode {
        return;
    }
    tokio::spawn(async move {
        loop {
            match run_once(&state).await {
                Ok((invoices, suspended, ended)) if invoices + suspended + ended > 0 => {
                    tracing::info!(invoices, suspended, ended, "billing run finished");
                }
                Ok(_) => {}
                Err(error) => tracing::warn!(?error, "billing run failed"),
            }
            tokio::time::sleep(RUN_EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::tax_included;

    #[test]
    fn splits_vat_out_of_gross_prices() {
        assert_eq!(tax_included(650, 2100), 113);
        assert_eq!(tax_included(1250, 2100), 217);
        assert_eq!(tax_included(5100, 2100), 885);
        assert_eq!(tax_included(1000, 0), 0);
    }
}
