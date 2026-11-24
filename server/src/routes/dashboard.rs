use axum::Json;
use axum::extract::State;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;

const EXPIRY_WINDOW_DAYS: i32 = 30;

#[derive(Serialize, sqlx::FromRow)]
pub struct ActiveTicket {
    id: Uuid,
    number: i64,
    subject: String,
    status: String,
    department: String,
    #[serde(with = "time::serde::rfc3339")]
    last_reply_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ExpiringService {
    kind: String,
    reference: String,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct Dashboard {
    active_tickets: Vec<ActiveTicket>,
    expiring_soon: Vec<ExpiringService>,
}

pub async fn show(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Dashboard>> {
    let active_tickets = sqlx::query_as(
        "SELECT t.id, t.number, t.subject, t.status, d.name AS department, t.last_reply_at
         FROM tickets t JOIN departments d ON d.id = t.department_id
         WHERE t.user_id = $1 AND t.status <> 'closed'
         ORDER BY t.last_reply_at DESC LIMIT 5",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;

    let expiring_soon = sqlx::query_as(
        "SELECT 'subscription' AS kind, s.code AS reference,
                COALESCE((SELECT description FROM subscription_items i WHERE i.subscription_id = s.id ORDER BY description LIMIT 1), s.code) AS name,
                s.expires_at
         FROM subscriptions s
         WHERE s.user_id = $1 AND s.status = 'active' AND s.payment_mode = 'manual'
           AND s.expires_at < now() + make_interval(days => $2)
         ORDER BY s.expires_at",
    )
    .bind(user.id)
    .bind(EXPIRY_WINDOW_DAYS)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(Dashboard {
        active_tickets,
        expiring_soon,
    }))
}
