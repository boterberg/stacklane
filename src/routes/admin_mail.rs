use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentAdmin;
use crate::error::AppResult;
use crate::pagination::{PageQuery, Paginated, like_pattern};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct MailFilter {
    search: Option<String>,
    status: Option<String>,
    #[serde(flatten)]
    page: PageQuery,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct MailEntry {
    id: Uuid,
    recipient: String,
    subject: String,
    kind: String,
    status: String,
    error: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

pub async fn index(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Query(filter): Query<MailFilter>,
) -> AppResult<Json<Paginated<MailEntry>>> {
    let search = like_pattern(filter.search.as_deref());
    let statuses: Vec<&str> = filter
        .status
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|status| ["sent", "failed", "skipped"].contains(status))
        .collect();
    const WHERE: &str = "WHERE ($1::text IS NULL OR recipient ILIKE $1 OR subject ILIKE $1)
                           AND (cardinality($2::text[]) = 0 OR status = ANY($2))";
    let total: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT count(*) FROM mail_log {WHERE}"
    )))
    .bind(&search)
    .bind(&statuses)
    .fetch_one(&state.db)
    .await?;
    let rows = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT id, recipient, subject, kind, status, error, created_at FROM mail_log {WHERE}
         ORDER BY created_at DESC LIMIT $3 OFFSET $4"
    )))
    .bind(&search)
    .bind(&statuses)
    .bind(filter.page.per_page())
    .bind(filter.page.offset())
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Paginated::new(rows, total, &filter.page)))
}
