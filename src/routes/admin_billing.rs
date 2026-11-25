use axum::Json;
use axum::extract::{Path, Query, State};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use sqlx::AssertSqlSafe;
use time::Date;
use uuid::Uuid;

use crate::auth::CurrentAdmin;
use crate::error::{AppError, AppResult};
use crate::i18n::Locale;
use crate::pagination::{PageQuery, Paginated};
use crate::payments;
use crate::routes::billing::{InvoiceDetail, load_invoice, pdf_response};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct Filter {
    q: Option<String>,
    status: Option<String>,
    #[serde(flatten)]
    page: PageQuery,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AdminInvoice {
    id: i64,
    status: String,
    #[serde(with = "crate::dates::day")]
    issued_at: Date,
    #[serde(with = "crate::dates::day")]
    due_at: Date,
    total_cents: i32,
    currency: String,
    customer_name: String,
    customer_email: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct PaymentRow {
    provider: String,
    status: String,
    amount_cents: i32,
    method: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: time::OffsetDateTime,
}

#[derive(Serialize)]
pub struct AdminInvoiceDetail {
    #[serde(flatten)]
    invoice: InvoiceDetail,
    customer_email: String,
    payments: Vec<PaymentRow>,
}

pub async fn index(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Query(filter): Query<Filter>,
) -> AppResult<Json<Paginated<AdminInvoice>>> {
    let statuses: Vec<String> = filter
        .status
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|status| ["open", "paid", "cancelled", "refunded"].contains(status))
        .map(str::to_owned)
        .collect();
    let search = filter
        .q
        .as_deref()
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
        });
    let condition = "(cardinality($1::text[]) = 0 OR v.status = ANY($1))
        AND ($2::text IS NULL OR v.id::text ILIKE $2 OR u.email::text ILIKE $2
             OR (u.first_name || ' ' || u.last_name) ILIKE $2 OR u.company_name ILIKE $2)";
    let total: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT count(*) FROM invoices v JOIN users u ON u.id = v.user_id WHERE {condition}"
    )))
    .bind(&statuses)
    .bind(&search)
    .fetch_one(&state.db)
    .await?;
    let rows = sqlx::query_as(AssertSqlSafe(format!(
        "SELECT v.id, v.status, v.issued_at, v.due_at, v.total_cents, v.currency,
                COALESCE(NULLIF(trim(u.first_name || ' ' || u.last_name), ''), u.email::text) AS customer_name,
                u.email::text AS customer_email
         FROM invoices v JOIN users u ON u.id = v.user_id
         WHERE {condition} ORDER BY v.issued_at DESC, v.id DESC LIMIT $3 OFFSET $4"
    )))
    .bind(&statuses)
    .bind(&search)
    .bind(filter.page.per_page())
    .bind(filter.page.offset())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Paginated::new(rows, total, &filter.page)))
}

pub async fn show(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(id): Path<i64>,
) -> AppResult<Json<AdminInvoiceDetail>> {
    let invoice = load_invoice(&state, None, id).await?;
    let customer_email: String = sqlx::query_scalar(
        "SELECT u.email::text FROM invoices v JOIN users u ON u.id = v.user_id WHERE v.id = $1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    let payments = sqlx::query_as(
        "SELECT provider, status, amount_cents, method, created_at FROM payments WHERE invoice_id = $1 ORDER BY created_at DESC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(AdminInvoiceDetail {
        invoice,
        customer_email,
        payments,
    }))
}

pub async fn mark_paid(
    State(state): State<AppState>,
    admin: CurrentAdmin,
    Path(id): Path<i64>,
) -> AppResult<Json<AdminInvoiceDetail>> {
    let total: i32 =
        sqlx::query_scalar("SELECT total_cents FROM invoices WHERE id = $1 AND status = 'open'")
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or_else(|| AppError::field("invoice", "Deze factuur staat niet meer open."))?;
    let payment_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO payments (id, invoice_id, provider, status, amount_cents, method) VALUES ($1, $2, 'manual', 'open', $3, 'manual')",
    )
    .bind(payment_id)
    .bind(id)
    .bind(total)
    .execute(&state.db)
    .await?;
    payments::update_payment(&state, payment_id, "paid", Some("manual")).await?;
    tracing::info!(admin = %admin.id, invoice = id, "invoice marked as paid");
    show(State(state), admin, Path(id)).await
}

pub async fn cancel(
    State(state): State<AppState>,
    admin: CurrentAdmin,
    Path(id): Path<i64>,
) -> AppResult<Json<AdminInvoiceDetail>> {
    let updated =
        sqlx::query("UPDATE invoices SET status = 'cancelled' WHERE id = $1 AND status = 'open'")
            .bind(id)
            .execute(&state.db)
            .await?
            .rows_affected();
    if updated == 0 {
        return Err(AppError::field(
            "invoice",
            "Deze factuur staat niet meer open.",
        ));
    }
    show(State(state), admin, Path(id)).await
}

pub async fn pdf(
    State(state): State<AppState>,
    admin: CurrentAdmin,
    Path(id): Path<i64>,
) -> AppResult<impl IntoResponse> {
    let invoice = load_invoice(&state, None, id).await?;
    let locale: String = sqlx::query_scalar("SELECT locale FROM users WHERE id = $1")
        .bind(admin.id)
        .fetch_one(&state.db)
        .await?;
    pdf_response(&state, &invoice, Locale::parse(&locale)).await
}
