use axum::Json;
use axum::extract::State;
use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Notification {
    id: Uuid,
    title: String,
    body: String,
    link: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct NotificationList {
    unread_count: usize,
    items: Vec<Notification>,
}

pub async fn index(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<NotificationList>> {
    let items: Vec<Notification> = sqlx::query_as(
        "SELECT id, title, body, link, created_at FROM notifications
         WHERE user_id = $1 AND read_at IS NULL ORDER BY created_at DESC LIMIT 50",
    )
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(NotificationList {
        unread_count: items.len(),
        items,
    }))
}
