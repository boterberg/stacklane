use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::auth::{Staff, permission};
use crate::error::AppResult;
use crate::settings::{self, Billing};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct MonthTotal {
    month: String,
    revenue_cents: i64,
    new_customers: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct KindCount {
    kind: String,
    count: i64,
}

#[derive(Serialize)]
pub struct Stats {
    currency: String,
    monthly_recurring_cents: i64,
    revenue_this_month_cents: i64,
    revenue_last_month_cents: i64,
    open_invoices: i64,
    open_cents: i64,
    customers: i64,
    paying_customers: i64,
    new_customers_30_days: i64,
    months: Vec<MonthTotal>,
    services: Vec<KindCount>,
}

pub async fn show(
    State(state): State<AppState>,
    _admin: Staff<permission::Billing>,
) -> AppResult<Json<Stats>> {
    let billing: Billing = settings::get(&state.db, "billing").await?;
    let currency = billing.currency;
    let monthly_recurring_cents: i64 = sqlx::query_scalar(
        "SELECT COALESCE(sum(i.monthly_price_cents), 0)::bigint
         FROM subscriptions s JOIN subscription_items i ON i.subscription_id = s.id
         WHERE s.status = 'active'",
    )
    .fetch_one(&state.db)
    .await?;
    let (revenue_this_month_cents, revenue_last_month_cents, open_invoices, open_cents): (
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT
            COALESCE(sum(total_cents) FILTER (WHERE status = 'paid' AND paid_at >= date_trunc('month', now())), 0)::bigint,
            COALESCE(sum(total_cents) FILTER (WHERE status = 'paid' AND paid_at >= date_trunc('month', now()) - interval '1 month'
                                              AND paid_at < date_trunc('month', now())), 0)::bigint,
            count(*) FILTER (WHERE status = 'open'),
            COALESCE(sum(total_cents) FILTER (WHERE status = 'open'), 0)::bigint
         FROM invoices WHERE currency = $1",
    )
    .bind(&currency)
    .fetch_one(&state.db)
    .await?;
    let (customers, paying_customers, new_customers_30_days): (i64, i64, i64) = sqlx::query_as(
        "SELECT count(*),
                count(*) FILTER (WHERE EXISTS (SELECT 1 FROM subscriptions s WHERE s.user_id = u.id AND s.status = 'active')),
                count(*) FILTER (WHERE u.created_at >= now() - interval '30 days')
         FROM users u WHERE u.role = 'customer' AND u.deleted_at IS NULL",
    )
    .fetch_one(&state.db)
    .await?;
    let months = sqlx::query_as(
        "WITH months AS (
            SELECT generate_series(date_trunc('month', now()) - interval '11 months', date_trunc('month', now()), interval '1 month') AS month
         )
         SELECT to_char(m.month, 'YYYY-MM') AS month,
                COALESCE((SELECT sum(i.total_cents) FROM invoices i
                          WHERE i.status = 'paid' AND i.currency = $1
                            AND i.paid_at >= m.month AND i.paid_at < m.month + interval '1 month'), 0)::bigint AS revenue_cents,
                (SELECT count(*) FROM users u
                 WHERE u.role = 'customer' AND u.deleted_at IS NULL
                   AND u.created_at >= m.month AND u.created_at < m.month + interval '1 month') AS new_customers
         FROM months m ORDER BY m.month",
    )
    .bind(&currency)
    .fetch_all(&state.db)
    .await?;
    let services = sqlx::query_as(
        "SELECT kind, count(*) AS count FROM services WHERE status = 'active' GROUP BY kind ORDER BY count(*) DESC, kind",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Stats {
        currency,
        monthly_recurring_cents,
        revenue_this_month_cents,
        revenue_last_month_cents,
        open_invoices,
        open_cents,
        customers,
        paying_customers,
        new_customers_30_days,
        months,
        services,
    }))
}
