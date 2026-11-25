use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::AppResult;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Entry {
    id: Uuid,
    action: String,
    detail: Option<String>,
    actor: Option<String>,
    ip: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

pub async fn record(
    state: &AppState,
    service: Uuid,
    user: Uuid,
    ip: &str,
    action: &str,
    detail: Option<&str>,
) {
    let detail = detail.map(|detail| detail.chars().take(300).collect::<String>());
    let result = sqlx::query(
        "INSERT INTO service_activity (id, service_id, user_id, action, detail, ip)
         SELECT $1, $2, $3, $4, $5, $6
         WHERE $4 <> 'console' OR NOT EXISTS (
             SELECT 1 FROM service_activity
             WHERE service_id = $2 AND user_id = $3 AND action = 'console' AND created_at > now() - interval '10 minutes'
         )",
    )
    .bind(Uuid::new_v4())
    .bind(service)
    .bind(user)
    .bind(action)
    .bind(detail)
    .bind(ip)
    .execute(&state.db)
    .await;
    if let Err(error) = result {
        tracing::warn!(?error, action, "writing the activity log failed");
    }
}

pub async fn list(state: &AppState, service: Uuid) -> AppResult<Vec<Entry>> {
    Ok(sqlx::query_as(
        "SELECT a.id, a.action, a.detail, a.ip, a.created_at,
                COALESCE(NULLIF(trim(u.first_name || ' ' || u.last_name), ''), u.email::text) AS actor
         FROM service_activity a LEFT JOIN users u ON u.id = a.user_id
         WHERE a.service_id = $1
         ORDER BY a.created_at DESC LIMIT 200",
    )
    .bind(service)
    .fetch_all(&state.db)
    .await?)
}
