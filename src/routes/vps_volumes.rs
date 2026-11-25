use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::shop::MAX_VOLUMES;
use super::vps::machine_of;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, volumes};
use crate::routes::vps_access::VpsActor;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Volume {
    id: Uuid,
    name: String,
    status: String,
    size_gb: Option<i32>,
    server_id: Option<Uuid>,
    server_name: Option<String>,
    subscription_code: Option<String>,
    cancelled: bool,
    #[serde(serialize_with = "crate::error::public_error")]
    last_error: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct Overview {
    volumes: Vec<Volume>,
    used: i64,
    limit: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct VolumeOption {
    server_id: Uuid,
    server_name: String,
    product_id: Uuid,
    price_per_gb_cents: i32,
}

#[derive(Deserialize)]
pub struct Attach {
    server_id: Uuid,
}

pub async fn index(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Overview>> {
    actor.require("volumes.view")?;
    let volumes: Vec<Volume> = sqlx::query_as(
        "SELECT v.id, v.name, v.status, (v.details->>'size_gb')::int AS size_gb,
                s.id AS server_id, s.name AS server_name, sub.code AS subscription_code,
                sub.cancelled_at IS NOT NULL AS cancelled, v.last_error, v.created_at
         FROM services v
         LEFT JOIN services s ON s.id = (v.details->>'server_id')::uuid AND s.status <> 'terminated'
         LEFT JOIN subscriptions sub ON sub.id = v.subscription_id
         WHERE v.user_id = $1 AND v.kind = 'volume' AND v.status <> 'terminated'
         ORDER BY v.created_at",
    )
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    let used = i64::try_from(volumes.len()).unwrap_or(i64::MAX);
    Ok(Json(Overview {
        volumes,
        used,
        limit: MAX_VOLUMES,
    }))
}

pub async fn options(
    State(state): State<AppState>,
    actor: VpsActor,
) -> AppResult<Json<Vec<VolumeOption>>> {
    actor.require_owner()?;
    let candidates: Vec<VolumeOption> = sqlx::query_as(
        "SELECT DISTINCT ON (s.id) s.id AS server_id, s.name AS server_name, p.id AS product_id,
                p.monthly_price_cents AS price_per_gb_cents
         FROM services s
         JOIN products p ON p.kind = 'volume' AND p.is_active AND p.integration_id = s.integration_id
         WHERE s.user_id = $1 AND s.kind = 'vps' AND s.status = 'active'
         ORDER BY s.id, p.monthly_price_cents",
    )
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    let mut usable = Vec::new();
    for option in candidates {
        if let Ok(machine) = machine_of(&state, actor.owner, option.server_id).await
            && machine.features().volumes
        {
            usable.push(option);
        }
    }
    usable.sort_by(|a, b| a.server_name.cmp(&b.server_name));
    Ok(Json(usable))
}

struct Owned {
    integration_id: Uuid,
    external_id: String,
    details: Value,
}

async fn owned(state: &AppState, actor: &VpsActor, id: Uuid) -> AppResult<Owned> {
    let row: (Option<Uuid>, Option<String>, Value, String) = sqlx::query_as(
        "SELECT integration_id, external_id, details, status FROM services
         WHERE id = $1 AND user_id = $2 AND kind = 'volume' AND status <> 'terminated'",
    )
    .bind(id)
    .bind(actor.owner)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let not_ready = || AppError::field("volume", "Dit volume is nog niet klaar.");
    if row.3 != "active" {
        return Err(not_ready());
    }
    Ok(Owned {
        integration_id: row.0.ok_or_else(not_ready)?,
        external_id: row.1.ok_or_else(not_ready)?,
        details: row.2,
    })
}

async fn storage(state: &AppState, integration_id: Uuid) -> AppResult<volumes::Storage> {
    volumes::Storage::from_driver(integrations::driver(state, integration_id).await?)
        .ok_or_else(|| AppError::field("volume", "Dit platform kan geen volumes beheren."))
}

async fn server_external(state: &AppState, server_id: Option<&str>) -> AppResult<Option<String>> {
    let Some(server_id) = server_id.and_then(|id| Uuid::parse_str(id).ok()) else {
        return Ok(None);
    };
    Ok(sqlx::query_scalar(
        "SELECT external_id FROM services WHERE id = $1 AND status <> 'terminated'",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .flatten())
}

fn failed(error: anyhow::Error) -> AppError {
    AppError::upstream("volume", error)
}

fn reference(volume: &Owned) -> volumes::VolumeRef {
    volumes::VolumeRef {
        host: volume.details["host"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        id: volume.external_id.clone(),
    }
}

pub async fn detach(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    actor.require("volumes.detach")?;
    let _guard = crate::locks::service(&state.db, id).await?;
    let volume = owned(&state, &actor, id).await?;
    let Some(server) = server_external(&state, volume.details["server_id"].as_str()).await? else {
        return Err(AppError::field(
            "volume",
            "Dit volume zit niet aan een server vast.",
        ));
    };
    storage(&state, volume.integration_id)
        .await?
        .detach(&reference(&volume), &server)
        .await
        .map_err(failed)?;
    sqlx::query(
        "UPDATE services SET details = details - 'server_id', updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn attach(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<Attach>,
) -> AppResult<StatusCode> {
    actor.require("volumes.attach")?;
    let _guard = crate::locks::service(&state.db, id).await?;
    let volume = owned(&state, &actor, id).await?;
    if volume.details["server_id"].as_str().is_some() {
        return Err(AppError::field(
            "volume",
            "Koppel het volume eerst los van de huidige server.",
        ));
    }
    let target: Option<(Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT integration_id, external_id FROM services
         WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status = 'active'",
    )
    .bind(input.server_id)
    .bind(actor.owner)
    .fetch_optional(&state.db)
    .await?;
    let Some((Some(integration_id), Some(server))) = target else {
        return Err(AppError::field(
            "server_id",
            "Kies een van je actieve servers.",
        ));
    };
    if integration_id != volume.integration_id {
        return Err(AppError::field(
            "server_id",
            "Dit volume kan niet bij deze server. Kies een server op hetzelfde platform.",
        ));
    }
    let moved = storage(&state, volume.integration_id)
        .await?
        .attach(&reference(&volume), &server)
        .await
        .map_err(failed)?;
    sqlx::query(
        "UPDATE services SET external_id = $2,
                details = details || jsonb_build_object('server_id', $3::text, 'host', $4::text),
                updated_at = now()
         WHERE id = $1",
    )
    .bind(id)
    .bind(&moved.id)
    .bind(input.server_id.to_string())
    .bind(&moved.host)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
