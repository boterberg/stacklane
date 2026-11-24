use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, webhosting};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Website {
    id: Uuid,
    name: String,
    status: String,
    product_name: Option<String>,
    details: Value,
    #[serde(serialize_with = "crate::error::public_error")]
    last_error: Option<String>,
    subscription_code: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

const SELECT: &str =
    "SELECT s.id, s.name, s.status, p.name AS product_name, s.details, s.last_error,
        sub.code AS subscription_code, sub.expires_at, s.created_at
     FROM services s
     LEFT JOIN products p ON p.id = s.product_id
     LEFT JOIN subscriptions sub ON sub.id = s.subscription_id
     WHERE s.kind = 'webhosting' AND s.status <> 'terminated' AND s.user_id = $1";

pub async fn index(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<Vec<Website>>> {
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} ORDER BY s.name")))
            .bind(user.id)
            .fetch_all(&state.db)
            .await?,
    ))
}

pub async fn show(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Website>> {
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND s.id = $2")))
            .bind(user.id)
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?,
    ))
}

#[derive(Serialize)]
pub struct PanelLink {
    url: String,
}

pub async fn panel_login(
    State(state): State<AppState>,
    user: CurrentUser,
    crate::limits::ClientIp(ip): crate::limits::ClientIp,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PanelLink>> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    let (integration_id, external_id, status): (Option<Uuid>, Option<String>, String) =
        sqlx::query_as(
            "SELECT integration_id, external_id, status FROM services WHERE id = $1 AND user_id = $2 AND kind = 'webhosting'",
        )
        .bind(id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    let (Some(integration_id), Some(external_id), "active") =
        (integration_id, external_id, status.as_str())
    else {
        return Err(AppError::field(
            "website",
            "Dit webhostingpakket is nog niet klaar.",
        ));
    };
    let driver = integrations::driver(&state, integration_id).await?;
    let hosting = webhosting::Hosting::from_driver(&driver).ok_or(AppError::NotFound)?;
    let url = hosting
        .login_url(&external_id, &ip)
        .await
        .map_err(|error| AppError::upstream("website", error))?;
    Ok(Json(PanelLink { url }))
}
