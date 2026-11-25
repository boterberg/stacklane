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
    pub(crate) currency: String,
}

#[derive(Serialize)]
pub struct SubscriptionDetail {
    #[serde(flatten)]
    summary: SubscriptionSummary,
    #[serde(with = "time::serde::rfc3339")]
    started_at: OffsetDateTime,
    mandate_id: Option<Uuid>,
    mandate: Option<String>,
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
    pub(crate) vat_scheme: String,
    pub(crate) vat_country: Option<String>,
    pub(crate) customer_vat_number: Option<String>,
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
    let (started_at, mandate_id, mandate): (OffsetDateTime, Option<Uuid>, Option<String>) =
        sqlx::query_as(
            "SELECT s.started_at, m.id, m.description FROM subscriptions s
         LEFT JOIN payment_mandates m ON m.id = s.mandate_id AND m.revoked_at IS NULL
         WHERE s.id = $1",
        )
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
        "SELECT DISTINCT v.id, v.status, v.issued_at, v.due_at, v.total_cents, v.currency
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
        mandate_id,
        mandate,
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
        "SELECT v.id, v.status, v.issued_at, v.due_at, v.total_cents, v.currency FROM invoices v
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
    currency: String,
    tax_cents: i32,
    tax_rate_basis_points: i32,
    vat_scheme: String,
    vat_country: Option<String>,
    customer_vat_number: Option<String>,
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
        "SELECT id, status, issued_at, due_at, total_cents, currency, tax_cents, tax_rate_basis_points, vat_scheme, vat_country::text,
                customer_vat_number, paid_at, billing_snapshot, user_id
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
            currency: row.currency,
        },
        tax_cents: row.tax_cents,
        tax_rate_basis_points: row.tax_rate_basis_points,
        vat_scheme: row.vat_scheme,
        vat_country: row.vat_country,
        customer_vat_number: row.customer_vat_number,
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

async fn owned_subscription(
    state: &AppState,
    user: &CurrentUser,
    code: &str,
) -> AppResult<(Uuid, String, bool, OffsetDateTime)> {
    sqlx::query_as("SELECT id, status, cancelled_at IS NOT NULL, expires_at FROM subscriptions WHERE user_id = $1 AND code = $2")
        .bind(user.id)
        .bind(code)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn cancel_subscription(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(code): Path<String>,
) -> AppResult<Json<SubscriptionDetail>> {
    let (id, status, cancelled, _) = owned_subscription(&state, &user, &code).await?;
    if cancelled || !matches!(status.as_str(), "active" | "pending" | "suspended") {
        return Err(AppError::field(
            "subscription",
            "Dit abonnement is al opgezegd.",
        ));
    }
    let mut transaction = state.db.begin().await?;
    crate::locks::transaction(&mut transaction, format!("subscription {id}")).await?;
    if status == "pending" {
        withdraw_order(&mut transaction, id).await?;
    } else {
        sqlx::query("UPDATE subscriptions SET cancelled_at = now() WHERE id = $1")
            .bind(id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "UPDATE invoices v SET status = 'cancelled' WHERE v.status = 'open'
               AND EXISTS (SELECT 1 FROM invoice_lines l JOIN subscriptions s ON s.id = l.subscription_id
                           WHERE l.invoice_id = v.id AND s.id = $1
                             AND l.period_start >= GREATEST(s.expires_at::date, CURRENT_DATE))",
        )
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    subscription(State(state), user, Path(code)).await
}

async fn withdraw_order(connection: &mut sqlx::PgConnection, id: Uuid) -> AppResult<()> {
    let invoices: Vec<i64> = sqlx::query_scalar(
        "UPDATE invoices v SET status = 'cancelled' WHERE v.status = 'open'
           AND EXISTS (SELECT 1 FROM invoice_lines l WHERE l.invoice_id = v.id AND l.subscription_id = $1)
         RETURNING v.id",
    )
    .bind(id)
    .fetch_all(&mut *connection)
    .await?;
    let withdrawn: Vec<Uuid> = sqlx::query_scalar(
        "UPDATE subscriptions s SET status = 'expired', cancelled_at = now()
         WHERE s.status = 'pending'
           AND (s.id = $1 OR EXISTS (SELECT 1 FROM invoice_lines l
                                     WHERE l.subscription_id = s.id AND l.invoice_id = ANY($2)))
         RETURNING s.id",
    )
    .bind(id)
    .bind(&invoices)
    .fetch_all(&mut *connection)
    .await?;
    sqlx::query(
        "UPDATE services SET status = 'terminated', updated_at = now()
         WHERE subscription_id = ANY($1) AND status = 'pending'",
    )
    .bind(&withdrawn)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub async fn resume_subscription(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(code): Path<String>,
) -> AppResult<Json<SubscriptionDetail>> {
    let (id, status, cancelled, expires_at) = owned_subscription(&state, &user, &code).await?;
    if !cancelled
        || !matches!(status.as_str(), "active" | "pending" | "suspended")
        || expires_at < OffsetDateTime::now_utc()
    {
        return Err(AppError::field(
            "subscription",
            "Dit abonnement kun je niet meer hervatten.",
        ));
    }
    sqlx::query("UPDATE subscriptions SET cancelled_at = NULL WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    subscription(State(state), user, Path(code)).await
}

#[derive(Deserialize)]
pub struct PaymentMethod {
    mandate_id: Option<Uuid>,
}

pub async fn set_payment_method(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(code): Path<String>,
    Json(input): Json<PaymentMethod>,
) -> AppResult<Json<SubscriptionDetail>> {
    let (id, _, _, _) = owned_subscription(&state, &user, &code).await?;
    if let Some(mandate_id) = input.mandate_id {
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
    }
    let mut transaction = state.db.begin().await?;
    crate::locks::transaction(&mut transaction, format!("subscription {id}")).await?;
    sqlx::query(
        "UPDATE subscriptions SET mandate_id = $2, payment_mode = CASE WHEN $2::uuid IS NULL THEN 'manual' ELSE 'automatic' END WHERE id = $1",
    )
    .bind(id)
    .bind(input.mandate_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    subscription(State(state), user, Path(code)).await
}
