use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::AssertSqlSafe;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::{CurrentAdmin, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::html::{sanitize_rich_text, visible_text};
use crate::pagination::{PageQuery, Paginated};
use crate::state::AppState;

const STATUSES: [&str; 5] = [
    "open",
    "answered",
    "awaiting_reply",
    "in_progress",
    "closed",
];
const MAX_SUBJECT_LENGTH: usize = 150;
const MAX_BODY_LENGTH: usize = 50_000;

#[derive(Clone, Copy)]
enum Scope {
    Customer(Uuid),
    Staff,
}

impl Scope {
    fn owner(self) -> Option<Uuid> {
        match self {
            Self::Customer(user_id) => Some(user_id),
            Self::Staff => None,
        }
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Department {
    id: Uuid,
    name: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TicketSummary {
    id: Uuid,
    number: i64,
    subject: String,
    status: String,
    department: String,
    customer_name: String,
    customer_email: String,
    message_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    last_reply_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TicketMessage {
    id: Uuid,
    is_staff: bool,
    author_name: String,
    body_html: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct TicketDetail {
    #[serde(flatten)]
    ticket: TicketSummary,
    messages: Vec<TicketMessage>,
}

#[derive(Deserialize)]
pub struct TicketFilter {
    q: Option<String>,
    status: Option<String>,
    #[serde(flatten)]
    page: PageQuery,
}

#[derive(Deserialize)]
pub struct CreateTicket {
    department_id: Option<Uuid>,
    subject: String,
    body_html: String,
}

#[derive(Deserialize)]
pub struct CreateReply {
    body_html: String,
}

#[derive(Deserialize)]
pub struct UpdateStatus {
    status: String,
}

const SUMMARY_SELECT: &str = "SELECT t.id, t.number, t.subject, t.status, d.name AS department,
        COALESCE(NULLIF(trim(u.first_name || ' ' || u.last_name), ''), u.email::text) AS customer_name,
        u.email::text AS customer_email,
        (SELECT count(*) FROM ticket_messages m WHERE m.ticket_id = t.id) AS message_count,
        t.created_at, t.last_reply_at
    FROM tickets t
    JOIN departments d ON d.id = t.department_id
    JOIN users u ON u.id = t.user_id";

fn clean_body(body_html: &str) -> Result<String, AppError> {
    if body_html.len() > MAX_BODY_LENGTH {
        return Err(AppError::field("body_html", "Je bericht is te lang."));
    }
    let cleaned = sanitize_rich_text(body_html);
    if visible_text(&cleaned).is_empty() {
        return Err(AppError::field("body_html", "Schrijf een bericht."));
    }
    Ok(cleaned)
}

fn requested_statuses(raw: Option<&str>) -> Vec<String> {
    match raw {
        None => STATUSES
            .iter()
            .filter(|status| **status != "closed")
            .map(|status| status.to_string())
            .collect(),
        Some(list) => list
            .split(',')
            .map(str::trim)
            .filter(|status| STATUSES.contains(status))
            .map(str::to_owned)
            .collect(),
    }
}

async fn load_summary(state: &AppState, scope: Scope, number: i64) -> AppResult<TicketSummary> {
    sqlx::query_as(AssertSqlSafe(format!(
        "{SUMMARY_SELECT} WHERE ($1::uuid IS NULL OR t.user_id = $1) AND t.number = $2"
    )))
    .bind(scope.owner())
    .bind(number)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)
}

async fn load_detail(state: &AppState, scope: Scope, number: i64) -> AppResult<TicketDetail> {
    let ticket = load_summary(state, scope, number).await?;
    let messages = sqlx::query_as(
        "SELECT m.id, m.is_staff,
                COALESCE(NULLIF(trim(u.first_name || ' ' || u.last_name), ''), u.email::text, 'Support') AS author_name,
                m.body_html, m.created_at
         FROM ticket_messages m LEFT JOIN users u ON u.id = m.author_user_id
         WHERE m.ticket_id = $1 ORDER BY m.created_at",
    )
    .bind(ticket.id)
    .fetch_all(&state.db)
    .await?;
    Ok(TicketDetail { ticket, messages })
}

async fn list(
    state: &AppState,
    scope: Scope,
    filter: &TicketFilter,
) -> AppResult<Paginated<TicketSummary>> {
    let statuses = requested_statuses(filter.status.as_deref());
    let search = filter
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(|q| format!("%{q}%"));
    let condition = "($1::uuid IS NULL OR t.user_id = $1) AND t.status = ANY($2)
        AND ($3::text IS NULL OR t.subject ILIKE $3 OR u.email::text ILIKE $3
             OR t.number::text = trim(both '%' from $3))";

    let total: i64 = sqlx::query_scalar(AssertSqlSafe(format!(
        "SELECT count(*) FROM tickets t JOIN users u ON u.id = t.user_id WHERE {condition}"
    )))
    .bind(scope.owner())
    .bind(&statuses)
    .bind(&search)
    .fetch_one(&state.db)
    .await?;

    let tickets = sqlx::query_as(AssertSqlSafe(format!(
        "{SUMMARY_SELECT} WHERE {condition} ORDER BY t.last_reply_at DESC LIMIT $4 OFFSET $5"
    )))
    .bind(scope.owner())
    .bind(&statuses)
    .bind(&search)
    .bind(filter.page.per_page())
    .bind(filter.page.offset())
    .fetch_all(&state.db)
    .await?;

    Ok(Paginated::new(tickets, total, &filter.page))
}

async fn add_message(
    state: &AppState,
    ticket_id: Uuid,
    author: Uuid,
    is_staff: bool,
    body: String,
) -> AppResult<()> {
    let next_status = if is_staff { "answered" } else { "open" };
    let mut transaction = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO ticket_messages (id, ticket_id, author_user_id, is_staff, body_html) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(ticket_id)
    .bind(author)
    .bind(is_staff)
    .bind(body)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("UPDATE tickets SET status = $2, last_reply_at = now() WHERE id = $1")
        .bind(ticket_id)
        .bind(next_status)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn departments(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> AppResult<Json<Vec<Department>>> {
    let departments = sqlx::query_as("SELECT id, name FROM departments ORDER BY position, name")
        .fetch_all(&state.db)
        .await?;
    Ok(Json(departments))
}

pub async fn index(
    State(state): State<AppState>,
    user: CurrentUser,
    Query(filter): Query<TicketFilter>,
) -> AppResult<Json<Paginated<TicketSummary>>> {
    Ok(Json(list(&state, Scope::Customer(user.id), &filter).await?))
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<CreateTicket>,
) -> AppResult<(StatusCode, Json<TicketDetail>)> {
    let subject = input.subject.trim().to_owned();
    let mut errors = BTreeMap::new();
    if input.department_id.is_none() {
        errors.insert("department_id", "Kies een afdeling.".to_owned());
    }
    if subject.chars().count() < 3 {
        errors.insert(
            "subject",
            "Vat je vraag samen in minstens 3 tekens.".to_owned(),
        );
    } else if subject.chars().count() > MAX_SUBJECT_LENGTH {
        errors.insert(
            "subject",
            format!("Gebruik maximaal {MAX_SUBJECT_LENGTH} tekens."),
        );
    }
    let body = match clean_body(&input.body_html) {
        Ok(body) => Some(body),
        Err(AppError::Validation(field_errors)) => {
            errors.extend(field_errors);
            None
        }
        Err(other) => return Err(other),
    };
    if let Some(department_id) = input.department_id {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM departments WHERE id = $1)")
                .bind(department_id)
                .fetch_one(&state.db)
                .await?;
        if !exists {
            errors.insert("department_id", "Deze afdeling bestaat niet.".to_owned());
        }
    }
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let mut transaction = state.db.begin().await?;
    let (ticket_id, number): (Uuid, i64) = sqlx::query_as(
        "INSERT INTO tickets (id, user_id, department_id, subject, status) VALUES ($1, $2, $3, $4, 'open')
         RETURNING id, number",
    )
    .bind(Uuid::new_v4())
    .bind(user.id)
    .bind(input.department_id)
    .bind(&subject)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query("INSERT INTO ticket_messages (id, ticket_id, author_user_id, is_staff, body_html) VALUES ($1, $2, $3, FALSE, $4)")
        .bind(Uuid::new_v4())
        .bind(ticket_id)
        .bind(user.id)
        .bind(body)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(load_detail(&state, Scope::Customer(user.id), number).await?),
    ))
}

pub async fn show(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(number): Path<i64>,
) -> AppResult<Json<TicketDetail>> {
    Ok(Json(
        load_detail(&state, Scope::Customer(user.id), number).await?,
    ))
}

pub async fn reply(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(number): Path<i64>,
    Json(input): Json<CreateReply>,
) -> AppResult<Json<TicketDetail>> {
    let scope = Scope::Customer(user.id);
    let ticket = load_summary(&state, scope, number).await?;
    add_message(
        &state,
        ticket.id,
        user.id,
        false,
        clean_body(&input.body_html)?,
    )
    .await?;
    Ok(Json(load_detail(&state, scope, number).await?))
}

pub async fn close(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(number): Path<i64>,
) -> AppResult<Json<TicketDetail>> {
    let scope = Scope::Customer(user.id);
    let ticket = load_summary(&state, scope, number).await?;
    sqlx::query("UPDATE tickets SET status = 'closed' WHERE id = $1")
        .bind(ticket.id)
        .execute(&state.db)
        .await?;
    Ok(Json(load_detail(&state, scope, number).await?))
}

pub async fn staff_index(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Query(filter): Query<TicketFilter>,
) -> AppResult<Json<Paginated<TicketSummary>>> {
    Ok(Json(list(&state, Scope::Staff, &filter).await?))
}

pub async fn staff_show(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(number): Path<i64>,
) -> AppResult<Json<TicketDetail>> {
    Ok(Json(load_detail(&state, Scope::Staff, number).await?))
}

pub async fn staff_reply(
    State(state): State<AppState>,
    admin: CurrentAdmin,
    Path(number): Path<i64>,
    Json(input): Json<CreateReply>,
) -> AppResult<Json<TicketDetail>> {
    let ticket = load_summary(&state, Scope::Staff, number).await?;
    add_message(
        &state,
        ticket.id,
        admin.id,
        true,
        clean_body(&input.body_html)?,
    )
    .await?;
    Ok(Json(load_detail(&state, Scope::Staff, number).await?))
}

pub async fn staff_update_status(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(number): Path<i64>,
    Json(input): Json<UpdateStatus>,
) -> AppResult<Json<TicketDetail>> {
    if !STATUSES.contains(&input.status.as_str()) {
        return Err(AppError::field("status", "Onbekende status."));
    }
    let ticket = load_summary(&state, Scope::Staff, number).await?;
    sqlx::query("UPDATE tickets SET status = $2 WHERE id = $1")
        .bind(ticket.id)
        .bind(&input.status)
        .execute(&state.db)
        .await?;
    Ok(Json(load_detail(&state, Scope::Staff, number).await?))
}

#[cfg(test)]
mod tests {
    use super::requested_statuses;

    #[test]
    fn default_filter_hides_closed_tickets() {
        assert_eq!(
            requested_statuses(None),
            ["open", "answered", "awaiting_reply", "in_progress"]
        );
    }

    #[test]
    fn unknown_statuses_are_ignored() {
        assert_eq!(
            requested_statuses(Some("closed, bogus,open")),
            ["closed", "open"]
        );
        assert!(requested_statuses(Some("")).is_empty());
    }
}
