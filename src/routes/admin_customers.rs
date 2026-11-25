use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::{Staff, permission};
use crate::error::{AppError, AppResult};
use crate::pagination::{PageQuery, Paginated, like_pattern};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CustomerFilter {
    search: Option<String>,
    #[serde(flatten)]
    page: PageQuery,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CustomerRow {
    id: Uuid,
    name: String,
    email: String,
    company_name: Option<String>,
    active_subscriptions: i64,
    active_services: i64,
    open_invoices: i64,
    open_cents: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Profile {
    id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    company_name: Option<String>,
    is_business: bool,
    coc_number: Option<String>,
    tax_id: Option<String>,
    phone: Option<String>,
    address_line1: Option<String>,
    address_line2: Option<String>,
    postal_code: Option<String>,
    city: Option<String>,
    country: String,
    locale: String,
    two_factor: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    email_verified_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Subscription {
    id: Uuid,
    code: String,
    status: String,
    description: String,
    billing_interval: String,
    payment_mode: String,
    monthly_cents: i64,
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Service {
    id: Uuid,
    kind: String,
    name: String,
    status: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Invoice {
    id: i64,
    status: String,
    total_cents: i32,
    #[serde(with = "crate::dates::day")]
    issued_at: time::Date,
    #[serde(with = "crate::dates::day")]
    due_at: time::Date,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Ticket {
    number: i64,
    subject: String,
    status: String,
    #[serde(with = "time::serde::rfc3339")]
    last_reply_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct CustomerDetail {
    profile: Profile,
    subscriptions: Vec<Subscription>,
    services: Vec<Service>,
    invoices: Vec<Invoice>,
    tickets: Vec<Ticket>,
}

const CUSTOMER_WHERE: &str = "WHERE u.role = 'customer' AND u.deleted_at IS NULL
    AND ($1::text IS NULL OR u.email::text ILIKE $1 OR (u.first_name || ' ' || u.last_name) ILIKE $1
         OR COALESCE(u.company_name, '') ILIKE $1)";

pub async fn index(
    State(state): State<AppState>,
    _admin: Staff<permission::Customers>,
    Query(filter): Query<CustomerFilter>,
) -> AppResult<Json<Paginated<CustomerRow>>> {
    let search = like_pattern(filter.search.as_deref());
    let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM users u {CUSTOMER_WHERE}"
    )))
    .bind(&search)
    .fetch_one(&state.db)
    .await?;
    let rows = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT u.id, COALESCE(NULLIF(trim(u.first_name || ' ' || u.last_name), ''), u.email::text) AS name,
                u.email::text AS email, u.company_name,
                (SELECT count(*) FROM subscriptions s WHERE s.user_id = u.id AND s.status = 'active') AS active_subscriptions,
                (SELECT count(*) FROM services s WHERE s.user_id = u.id AND s.status = 'active') AS active_services,
                (SELECT count(*) FROM invoices i WHERE i.user_id = u.id AND i.status = 'open') AS open_invoices,
                (SELECT COALESCE(sum(i.total_cents), 0) FROM invoices i WHERE i.user_id = u.id AND i.status = 'open')::bigint AS open_cents,
                u.created_at
         FROM users u {CUSTOMER_WHERE}
         ORDER BY u.created_at DESC LIMIT $2 OFFSET $3"
    )))
    .bind(&search)
    .bind(filter.page.per_page())
    .bind(filter.page.offset())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Paginated::new(rows, total, &filter.page)))
}

pub async fn show(
    State(state): State<AppState>,
    _admin: Staff<permission::Customers>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<CustomerDetail>> {
    let profile: Profile = sqlx::query_as(
        "SELECT id, email::text AS email, first_name, last_name, company_name, is_business, coc_number, tax_id,
                phone, address_line1, address_line2, postal_code, city, country::text AS country, locale,
                two_factor_enabled_at IS NOT NULL AS two_factor, email_verified_at, created_at
         FROM users WHERE id = $1 AND role = 'customer' AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let subscriptions = sqlx::query_as(
        "SELECT s.id, s.code, s.status, s.billing_interval, s.payment_mode, s.expires_at,
                COALESCE(string_agg(i.description, ', ' ORDER BY i.description), '') AS description,
                COALESCE(sum(i.monthly_price_cents), 0)::bigint AS monthly_cents
         FROM subscriptions s LEFT JOIN subscription_items i ON i.subscription_id = s.id
         WHERE s.user_id = $1
         GROUP BY s.id
         ORDER BY (s.status = 'active') DESC, s.expires_at DESC
         LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let services = sqlx::query_as(
        "SELECT id, kind, name, status, created_at FROM services
         WHERE user_id = $1 AND status <> 'terminated'
         ORDER BY created_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let invoices = sqlx::query_as(
        "SELECT id, status, total_cents, issued_at, due_at FROM invoices
         WHERE user_id = $1 ORDER BY issued_at DESC, id DESC LIMIT 10",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let tickets = sqlx::query_as(
        "SELECT number, subject, status, last_reply_at FROM tickets
         WHERE user_id = $1 ORDER BY last_reply_at DESC LIMIT 10",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(CustomerDetail {
        profile,
        subscriptions,
        services,
        invoices,
        tickets,
    }))
}
