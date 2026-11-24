use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use sqlx::AssertSqlSafe;
use time::{Date, OffsetDateTime};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::i18n::Locale;
use crate::invoice_pdf::render_invoice;
use crate::pagination::{PageQuery, Paginated};
use crate::settings::{self, Company};
use crate::state::AppState;

const SUBSCRIPTION_STATUSES: [&str; 5] = ["pending", "active", "suspended", "cancelled", "expired"];

#[derive(Deserialize)]
pub struct ListFilter {
    q: Option<String>,
    status: Option<String>,
    #[serde(flatten)]
    page: PageQuery,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct SubscriptionSummary {
    id: Uuid,
    code: String,
    name: String,
    status: String,
    billing_interval: String,
    payment_mode: String,
    monthly_total_cents: i64,
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    cancelled_at: Option<OffsetDateTime>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct SubscriptionItem {
    description: String,
    specs_line: Option<String>,
    monthly_price_cents: i32,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct InvoiceSummary {
    pub(crate) id: i64,
    pub(crate) status: String,
    #[serde(with = "crate::dates::day")]
    pub(crate) issued_at: Date,
    #[serde(with = "crate::dates::day")]
    pub(crate) due_at: Date,
    pub(crate) total_cents: i32,
}

#[derive(Serialize)]
pub struct SubscriptionDetail {
    #[serde(flatten)]
    summary: SubscriptionSummary,
    #[serde(with = "time::serde::rfc3339")]
    started_at: OffsetDateTime,
    items: Vec<SubscriptionItem>,
    invoices: Vec<InvoiceSummary>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct InvoiceLine {
    pub(crate) description: String,
    #[serde(with = "crate::dates::day::option")]
    pub(crate) period_start: Option<Date>,
    #[serde(with = "crate::dates::day::option")]
    pub(crate) period_end: Option<Date>,
    pub(crate) amount_cents: i32,
    pub(crate) subscription_code: Option<String>,
}

#[derive(Serialize)]
pub struct InvoiceDetail {
    #[serde(flatten)]
    pub(crate) summary: InvoiceSummary,
    pub(crate) tax_cents: i32,
    pub(crate) tax_rate_basis_points: i32,
    #[serde(with = "time::serde::rfc3339::option")]
    pub(crate) paid_at: Option<OffsetDateTime>,
    pub(crate) customer: serde_json::Value,
    pub(crate) lines: Vec<InvoiceLine>,
}

const SUBSCRIPTION_SELECT: &str = "SELECT s.id, s.code,
        COALESCE((SELECT i.description FROM subscription_items i WHERE i.subscription_id = s.id ORDER BY i.description LIMIT 1), s.code) AS name,
        s.status, s.billing_interval, s.payment_mode,
        COALESCE((SELECT sum(i.monthly_price_cents) FROM subscription_items i WHERE i.subscription_id = s.id), 0)::bigint AS monthly_total_cents,
        s.expires_at, s.cancelled_at
    FROM subscriptions s";

fn statuses(filter: Option<&str>) -> Vec<String> {
    filter
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|status| SUBSCRIPTION_STATUSES.contains(status))
        .map(str::to_owned)
        .collect()
}

fn search(query: Option<&str>) -> Option<String> {
    query
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .map(|query| {
            format!(
                "%{}%",
                query
                    .replace('\\', "\\\\")
                    .replace('%', "\\%")
                    .replace('_', "\\_")
            )
        })
}

pub async fn subscriptions(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(filter): Query<ListFilter>,
) -> AppResult<Json<Paginated<SubscriptionSummary>>> {
    let statuses = statuses(filter.status.as_deref());
    let search = search(filter.q.as_deref());
    let condition = "s.user_id = $1
        AND (cardinality($2::text[]) = 0 OR s.status = ANY($2))
        AND ($3::text IS NULL OR s.code ILIKE $3 OR EXISTS (
            SELECT 1 FROM subscription_items i WHERE i.subscription_id = s.id AND i.description ILIKE $3))";
    let total: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT count(*) FROM subscriptions s WHERE {condition}"
    )))
    .bind(user.id)
    .bind(&statuses)
    .bind(&search)
    .fetch_one(&state.db)
    .await?;
    let rows = sqlx::query_as(AssertSqlSafe(format!(
        "{SUBSCRIPTION_SELECT} WHERE {condition}
         ORDER BY (s.status IN ('active', 'pending', 'suspended')) DESC, s.expires_at DESC
         LIMIT $4 OFFSET $5"
    )))
    .bind(user.id)
    .bind(&statuses)
    .bind(&search)
    .bind(filter.page.per_page())
    .bind(filter.page.offset())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Paginated::new(rows, total, &filter.page)))
}

pub async fn subscription(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(code): Path<String>,
) -> AppResult<Json<SubscriptionDetail>> {
    let summary: SubscriptionSummary = sqlx::query_as(AssertSqlSafe(format!(
        "{SUBSCRIPTION_SELECT} WHERE s.user_id = $1 AND s.code = $2"
    )))
    .bind(user.id)
    .bind(&code)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let started_at: OffsetDateTime =
        sqlx::query_scalar("SELECT started_at FROM subscriptions WHERE id = $1")
            .bind(summary.id)
            .fetch_one(&state.db)
            .await?;
    let items = sqlx::query_as(
        "SELECT description, specs_line, monthly_price_cents FROM subscription_items
         WHERE subscription_id = $1 ORDER BY description",
    )
    .bind(summary.id)
    .fetch_all(&state.db)
    .await?;
    let invoices = sqlx::query_as(
        "SELECT DISTINCT v.id, v.status, v.issued_at, v.due_at, v.total_cents
         FROM invoices v JOIN invoice_lines l ON l.invoice_id = v.id
         WHERE v.user_id = $1 AND l.subscription_id = $2
         ORDER BY v.issued_at DESC, v.id DESC",
    )
    .bind(user.id)
    .bind(summary.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(SubscriptionDetail {
        summary,
        started_at,
        items,
        invoices,
    }))
}

pub async fn invoices(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(filter): Query<ListFilter>,
) -> AppResult<Json<Paginated<InvoiceSummary>>> {
    let search = search(filter.q.as_deref());
    let condition = "v.user_id = $1 AND ($2::text IS NULL OR v.id::text ILIKE $2 OR EXISTS (
            SELECT 1 FROM invoice_lines l WHERE l.invoice_id = v.id AND l.description ILIKE $2))";
    let total: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT count(*) FROM invoices v WHERE {condition}"
    )))
    .bind(user.id)
    .bind(&search)
    .fetch_one(&state.db)
    .await?;
    let rows = sqlx::query_as(AssertSqlSafe(format!(
        "SELECT v.id, v.status, v.issued_at, v.due_at, v.total_cents FROM invoices v
         WHERE {condition} ORDER BY v.issued_at DESC, v.id DESC LIMIT $3 OFFSET $4"
    )))
    .bind(user.id)
    .bind(&search)
    .bind(filter.page.per_page())
    .bind(filter.page.offset())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Paginated::new(rows, total, &filter.page)))
}

#[derive(sqlx::FromRow)]
struct InvoiceRow {
    id: i64,
    status: String,
    issued_at: Date,
    due_at: Date,
    total_cents: i32,
    tax_cents: i32,
    tax_rate_basis_points: i32,
    paid_at: Option<OffsetDateTime>,
    billing_snapshot: serde_json::Value,
    user_id: Uuid,
}

pub async fn load_invoice(
    state: &AppState,
    owner: Option<Uuid>,
    id: i64,
) -> AppResult<InvoiceDetail> {
    let row: InvoiceRow = sqlx::query_as(
        "SELECT id, status, issued_at, due_at, total_cents, tax_cents, tax_rate_basis_points, paid_at, billing_snapshot, user_id
         FROM invoices WHERE id = $1 AND ($2::uuid IS NULL OR user_id = $2)",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let snapshot = row.billing_snapshot;
    let user_id = row.user_id;
    let customer = if snapshot.get("name").is_some() {
        snapshot
    } else {
        sqlx::query_scalar(
            "SELECT jsonb_build_object(
                'name', COALESCE(NULLIF(trim(first_name || ' ' || last_name), ''), email::text),
                'company_name', company_name, 'address_line1', address_line1, 'address_line2', address_line2,
                'postal_code', postal_code, 'city', city, 'country', country::text)
             FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_one(&state.db)
        .await?
    };
    let lines = sqlx::query_as(
        "SELECT l.description, l.period_start, l.period_end, l.amount_cents, s.code AS subscription_code
         FROM invoice_lines l LEFT JOIN subscriptions s ON s.id = l.subscription_id
         WHERE l.invoice_id = $1 ORDER BY l.period_start NULLS LAST, l.description",
    )
    .bind(row.id)
    .fetch_all(&state.db)
    .await?;
    Ok(InvoiceDetail {
        summary: InvoiceSummary {
            id: row.id,
            status: row.status,
            issued_at: row.issued_at,
            due_at: row.due_at,
            total_cents: row.total_cents,
        },
        tax_cents: row.tax_cents,
        tax_rate_basis_points: row.tax_rate_basis_points,
        paid_at: row.paid_at,
        customer,
        lines,
    })
}

pub async fn invoice(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Json<InvoiceDetail>> {
    Ok(Json(load_invoice(&state, Some(user.id), id).await?))
}

pub async fn invoice_pdf(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let invoice = load_invoice(&state, Some(user.id), id).await?;
    let locale: String = sqlx::query_scalar("SELECT locale FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
    pdf_response(&state, &invoice, Locale::parse(&locale)).await
}

pub async fn pdf_response(
    state: &AppState,
    invoice: &InvoiceDetail,
    locale: Locale,
) -> AppResult<impl IntoResponse + use<>> {
    let company: Company = settings::get(&state.db, "company").await?;
    let body = render_invoice(invoice, &company, locale);
    let name = match locale {
        Locale::Nl => format!("factuur-{}.pdf", invoice.summary.id),
        Locale::En => format!("invoice-{}.pdf", invoice.summary.id),
    };
    Ok((
        [
            (header::CONTENT_TYPE, "application/pdf".to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{name}\""),
            ),
            (header::CACHE_CONTROL, "private, no-store".to_owned()),
        ],
        body,
    ))
}
