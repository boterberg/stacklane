use std::time::Duration;

use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::AppResult;
use crate::i18n::Locale;
use crate::invoicing::{self, Line};
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

pub(crate) async fn notify(
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
    let items: Vec<(String, i32, Option<i32>)> = sqlx::query_as(
        "SELECT description, monthly_price_cents, period_price_cents FROM subscription_items WHERE subscription_id = $1 ORDER BY description",
    )
    .bind(due.id)
    .fetch_all(&state.db)
    .await?;
    if items.is_empty() {
        return Ok(None);
    }
    let months = i64::from(interval_months(&due.billing_interval));
    let start = renewal_start(due.expires_at);
    let end = renewal_end(due.expires_at, &due.billing_interval);
    let lines: Vec<Line> = items
        .into_iter()
        .map(|(description, price, period_price)| Line {
            subscription_id: Some(due.id),
            description,
            period_start: Some(start),
            period_end: Some(end),
            amount_cents: period_price.map_or(i64::from(price) * months, i64::from),
        })
        .collect();
    let due_at = start.max(OffsetDateTime::now_utc().date());
    Ok(Some(
        invoicing::issue(state, due.user_id, &lines, due_at, billing).await?,
    ))
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
    let mut suspended = 0;
    for (subscription_id, user_id, code) in &overdue {
        let still_overdue: Option<Uuid> = sqlx::query_scalar(
            "UPDATE subscriptions s SET status = 'suspended', suspended_for_payment = TRUE
             WHERE s.id = $1 AND s.status = 'active' AND s.cancelled_at IS NULL
               AND s.expires_at + make_interval(days => $2) < now()
               AND EXISTS (
                   SELECT 1 FROM invoice_lines l JOIN invoices v ON v.id = l.invoice_id
                   WHERE l.subscription_id = s.id AND v.status = 'open')
             RETURNING s.id",
        )
        .bind(subscription_id)
        .bind(billing.suspend_after_days)
        .fetch_optional(&state.db)
        .await?;
        if still_overdue.is_none() {
            continue;
        }
        suspended += 1;
        let services: Vec<Uuid> = sqlx::query_scalar(
            "SELECT id FROM services WHERE subscription_id = $1 AND status = 'active'",
        )
        .bind(subscription_id)
        .fetch_all(&state.db)
        .await?;
        for service_id in services {
            if let Err(error) =
                crate::services::change_state_because(state, service_id, "suspend", "payment").await
            {
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
    Ok(suspended)
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
            if let Err(error) =
                crate::services::change_state_because(state, service_id, "suspend", "ended").await
            {
                tracing::warn!(%service_id, ?error, "suspending an ended service failed");
            }
        }
    }
    Ok(ended.len())
}

const ORDER_EXPIRES_AFTER_DAYS: i32 = 14;

async fn expire_orders(state: &AppState) -> AppResult<usize> {
    let mut transaction = state.db.begin().await?;
    let expired: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE subscriptions s SET status = 'expired'
         WHERE s.status = 'pending'
           AND EXISTS (SELECT 1 FROM invoice_lines l JOIN invoices i ON i.id = l.invoice_id
                       WHERE l.subscription_id = s.id AND i.status = 'open')
           AND NOT EXISTS (SELECT 1 FROM invoice_lines l JOIN invoices i ON i.id = l.invoice_id
                           WHERE l.subscription_id = s.id AND (i.status = 'paid' OR i.due_at >= CURRENT_DATE - $1))
         RETURNING s.id",
    )
    .bind(ORDER_EXPIRES_AFTER_DAYS)
    .fetch_all(&mut *transaction)
    .await?;
    if !expired.is_empty() {
        sqlx::query(
            "UPDATE services SET status = 'terminated', updated_at = now()
             WHERE subscription_id = ANY($1) AND status = 'pending'",
        )
        .bind(&expired)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE invoices i SET status = 'cancelled'
             WHERE i.status = 'open'
               AND EXISTS (SELECT 1 FROM invoice_lines l WHERE l.invoice_id = i.id AND l.subscription_id = ANY($1))
               AND NOT EXISTS (SELECT 1 FROM invoice_lines l WHERE l.invoice_id = i.id
                               AND (l.subscription_id IS NULL OR l.subscription_id <> ALL($1)))",
        )
        .bind(&expired)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(expired.len())
}

pub struct Run {
    pub invoices: usize,
    pub suspended: usize,
    pub ended: usize,
    pub expired: usize,
}

async fn recheck_vat_numbers(state: &AppState) -> AppResult<()> {
    let pending: Vec<(uuid::Uuid, String, String)> = sqlx::query_as(
        "SELECT id, tax_id, country::text FROM users
         WHERE is_business AND tax_id IS NOT NULL AND tax_id_verified_at IS NULL",
    )
    .fetch_all(&state.db)
    .await?;
    for (user_id, tax_id, country) in pending {
        let Some((code, number)) = crate::vat::split_vat_number(&tax_id, &country) else {
            continue;
        };
        if let Ok(true) = crate::vat::check_vies(&code, &number).await {
            sqlx::query(
                "UPDATE users SET tax_id_verified_at = now() WHERE id = $1 AND tax_id = $2",
            )
            .bind(user_id)
            .bind(&tax_id)
            .execute(&state.db)
            .await?;
        }
    }
    Ok(())
}

async fn reconcile_suspensions(state: &AppState) -> AppResult<()> {
    let wrong: Vec<(Uuid, &'static str, &'static str)> = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT v.id, CASE WHEN s.status = 'cancelled' THEN 'ended' ELSE 'payment' END
         FROM services v JOIN subscriptions s ON s.id = v.subscription_id
         WHERE v.status = 'active'
           AND (s.status = 'cancelled' OR (s.status = 'suspended' AND s.suspended_for_payment))",
    )
    .fetch_all(&state.db)
    .await?
    .into_iter()
    .map(|(id, reason)| {
        let reason = if reason == "ended" { "ended" } else { "payment" };
        (id, "suspend", reason)
    })
    .chain(
        sqlx::query_scalar::<_, Uuid>(
            "SELECT v.id FROM services v JOIN subscriptions s ON s.id = v.subscription_id
             WHERE v.status = 'suspended' AND v.suspension_reason = 'payment' AND s.status = 'active'",
        )
        .fetch_all(&state.db)
        .await?
        .into_iter()
        .map(|id| (id, "unsuspend", "payment")),
    )
    .collect();
    for (service_id, action, reason) in wrong {
        if let Err(error) =
            crate::services::change_state_because(state, service_id, action, reason).await
        {
            tracing::warn!(%service_id, action, ?error, "correcting a service state failed");
        }
    }
    Ok(())
}

pub async fn run_once(state: &AppState) -> AppResult<Run> {
    let mut lock = state.db.acquire().await?;
    let mine: bool =
        sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtext('stacklane billing run'))")
            .fetch_one(&mut *lock)
            .await?;
    if !mine {
        return Ok(Run {
            invoices: 0,
            suspended: 0,
            ended: 0,
            expired: 0,
        });
    }
    let run = run_locked(state).await;
    sqlx::query("SELECT pg_advisory_unlock(hashtext('stacklane billing run'))")
        .execute(&mut *lock)
        .await?;
    run
}

async fn run_locked(state: &AppState) -> AppResult<Run> {
    let billing: Billing = settings::get(&state.db, "billing").await?;
    recheck_vat_numbers(state).await?;
    crate::routes::vps_isos::remove_unused(state).await?;
    let run = Run {
        invoices: issue_renewals(state, &billing).await?,
        suspended: suspend_overdue(state, &billing).await?,
        ended: end_cancelled(state).await?,
        expired: expire_orders(state).await?,
    };
    reconcile_suspensions(state).await?;
    Ok(run)
}

pub fn spawn(state: AppState) {
    if state.config.demo_mode {
        return;
    }
    tokio::spawn(async move {
        loop {
            match run_once(&state).await {
                Ok(run) if run.invoices + run.suspended + run.ended + run.expired > 0 => {
                    tracing::info!(
                        invoices = run.invoices,
                        suspended = run.suspended,
                        ended = run.ended,
                        expired = run.expired,
                        "billing run finished"
                    );
                }
                Ok(_) => {}
                Err(error) => tracing::warn!(?error, "billing run failed"),
            }
            tokio::time::sleep(RUN_EVERY).await;
        }
    });
}
