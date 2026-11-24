use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::vps::machine_of;
use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, isos};
use crate::state::AppState;

pub const MAX_ISOS: i64 = 2;
const UNUSED_DAYS: i32 = 30;

#[derive(Serialize, sqlx::FromRow)]
pub struct Iso {
    id: Uuid,
    name: String,
    url: String,
    firmware: String,
    status: String,
    last_error: Option<String>,
    mounted_on: Option<Uuid>,
    server_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct Overview {
    isos: Vec<Iso>,
    used: i64,
    limit: i64,
    can_import: bool,
}

#[derive(Serialize)]
pub struct ServerIso {
    mounted: Option<Iso>,
    available: Vec<Iso>,
}

#[derive(Deserialize)]
pub struct NewIso {
    name: String,
    url: String,
    firmware: String,
}

#[derive(Deserialize)]
pub struct Mount {
    iso_id: Uuid,
}

const SELECT: &str = "SELECT i.id, i.name, i.url, i.firmware, i.status, i.last_error, i.mounted_on,
        s.name AS server_name, i.created_at
     FROM vps_isos i LEFT JOIN services s ON s.id = i.mounted_on
     WHERE i.user_id = $1";

async fn capable_server(state: &AppState, user_id: Uuid) -> AppResult<Option<Uuid>> {
    let servers: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM services WHERE user_id = $1 AND kind = 'vps' AND status = 'active' ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    for server in servers {
        if let Ok(machine) = machine_of(state, user_id, server).await
            && machine.features().volumes
        {
            return Ok(Some(server));
        }
    }
    Ok(None)
}

async fn store(state: &AppState, integration_id: Uuid) -> AppResult<isos::IsoStore> {
    isos::IsoStore::from_driver(integrations::driver(state, integration_id).await?)
        .ok_or_else(|| AppError::field("iso", "Dit platform kan geen ISO's gebruiken."))
}

pub async fn index(State(state): State<AppState>, user: CurrentUser) -> AppResult<Json<Overview>> {
    let isos: Vec<Iso> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} ORDER BY i.created_at"
    )))
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Overview {
        used: i64::try_from(isos.len()).unwrap_or(i64::MAX),
        isos,
        limit: MAX_ISOS,
        can_import: capable_server(&state, user.id).await?.is_some(),
    }))
}

async fn run_import(
    state: AppState,
    id: Uuid,
    integration_id: Uuid,
    name: String,
    url: String,
    uefi: bool,
) {
    let file = format!("stacklane-{}.iso", id.simple());
    let result = async {
        let store = store(&state, integration_id)
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        store.import(&file, &name, &url, uefi).await
    }
    .await;
    let (status, external_id, error) = match result {
        Ok(external_id) => ("ready", Some(external_id), None),
        Err(error) => {
            tracing::warn!(%id, error = %format!("{error:#}"), "importing an ISO failed");
            ("failed", None, Some(format!("{error:#}")))
        }
    };
    let _ = sqlx::query(
        "UPDATE vps_isos SET status = $2, external_id = $3, last_error = $4 WHERE id = $1",
    )
    .bind(id)
    .bind(status)
    .bind(external_id)
    .bind(error)
    .execute(&state.db)
    .await;
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<NewIso>,
) -> AppResult<(StatusCode, Json<Iso>)> {
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Geef de ISO een naam van hoogstens 60 tekens.",
        ));
    }
    if !matches!(input.firmware.as_str(), "bios" | "uefi") {
        return Err(AppError::field("firmware", "Kies BIOS of UEFI."));
    }
    let url = isos::check_url(&input.url)
        .await
        .map_err(|error| AppError::field("url", format!("{error:#}")))?;
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM vps_isos WHERE user_id = $1")
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
    if used >= MAX_ISOS {
        return Err(AppError::field(
            "name",
            "Je hebt al 2 ISO's. Verwijder er eerst een.",
        ));
    }
    let Some(server) = capable_server(&state, user.id).await? else {
        return Err(AppError::field(
            "name",
            "Je hebt minimaal één actieve VPS nodig voordat je ISO's kunt importeren.",
        ));
    };
    let integration_id: Uuid =
        sqlx::query_scalar("SELECT integration_id FROM services WHERE id = $1")
            .bind(server)
            .fetch_one(&state.db)
            .await?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO vps_isos (id, user_id, integration_id, name, url, firmware, status)
         VALUES ($1, $2, $3, $4, $5, $6, 'importing')",
    )
    .bind(id)
    .bind(user.id)
    .bind(integration_id)
    .bind(name)
    .bind(&url)
    .bind(&input.firmware)
    .execute(&state.db)
    .await?;
    tokio::spawn(run_import(
        state.clone(),
        id,
        integration_id,
        name.to_owned(),
        url,
        input.firmware == "uefi",
    ));
    let iso = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND i.id = $2")))
        .bind(user.id)
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    Ok((StatusCode::CREATED, Json(iso)))
}

pub async fn remove(state: &AppState, id: Uuid) -> anyhow::Result<()> {
    let (integration_id, external_id): (Uuid, Option<String>) =
        sqlx::query_as("SELECT integration_id, external_id FROM vps_isos WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    if let Some(external_id) = external_id {
        store(state, integration_id)
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?
            .delete(&external_id)
            .await?;
    }
    sqlx::query("DELETE FROM vps_isos WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(())
}

pub async fn destroy(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let row: Option<(String, Option<Uuid>)> =
        sqlx::query_as("SELECT status, mounted_on FROM vps_isos WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user.id)
            .fetch_optional(&state.db)
            .await?;
    match row {
        None => return Err(AppError::NotFound),
        Some((_, Some(_))) => {
            return Err(AppError::field(
                "iso",
                "Koppel de ISO eerst los van de server.",
            ));
        }
        Some((status, None)) if status == "importing" => {
            return Err(AppError::field(
                "iso",
                "Wacht tot de ISO klaar is met importeren.",
            ));
        }
        Some(_) => {}
    }
    remove(&state, id)
        .await
        .map_err(|error| AppError::field("iso", format!("{error:#}")))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn server_context(
    state: &AppState,
    user: &CurrentUser,
    id: Uuid,
) -> AppResult<(Uuid, String)> {
    machine_of(state, user.id, id).await?;
    let (integration_id, external_id): (Option<Uuid>, Option<String>) =
        sqlx::query_as("SELECT integration_id, external_id FROM services WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    match (integration_id, external_id) {
        (Some(integration_id), Some(external_id)) => Ok((integration_id, external_id)),
        _ => Err(AppError::NotFound),
    }
}

pub async fn server(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ServerIso>> {
    let integration_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT integration_id FROM services WHERE id = $1 AND user_id = $2 AND kind = 'vps'",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .flatten();
    let Some(integration_id) = integration_id else {
        return Err(AppError::NotFound);
    };
    let isos: Vec<Iso> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} AND i.integration_id = $2 ORDER BY i.created_at"
    )))
    .bind(user.id)
    .bind(integration_id)
    .fetch_all(&state.db)
    .await?;
    let (mounted, available): (Vec<Iso>, Vec<Iso>) =
        isos.into_iter().partition(|iso| iso.mounted_on == Some(id));
    Ok(Json(ServerIso {
        mounted: mounted.into_iter().next(),
        available: available
            .into_iter()
            .filter(|iso| iso.status == "ready" && iso.mounted_on.is_none())
            .collect(),
    }))
}

pub async fn mount(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<Mount>,
) -> AppResult<StatusCode> {
    let (integration_id, server) = server_context(&state, &user, id).await?;
    let already: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM vps_isos WHERE mounted_on = $1)")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    if already {
        return Err(AppError::field(
            "iso_id",
            "Koppel eerst de huidige ISO los.",
        ));
    }
    let iso: Option<(String, String)> = sqlx::query_as(
        "SELECT external_id, firmware FROM vps_isos
         WHERE id = $1 AND user_id = $2 AND integration_id = $3 AND status = 'ready' AND mounted_on IS NULL",
    )
    .bind(input.iso_id)
    .bind(user.id)
    .bind(integration_id)
    .fetch_optional(&state.db)
    .await?;
    let Some((external_id, firmware)) = iso else {
        return Err(AppError::field("iso_id", "Kies een ISO die klaar is."));
    };
    store(&state, integration_id)
        .await?
        .mount(&server, &external_id, firmware == "uefi")
        .await
        .map_err(|error| AppError::field("iso_id", format!("{error:#}")))?;
    sqlx::query("UPDATE vps_isos SET mounted_on = $2, last_used_at = now() WHERE id = $1")
        .bind(input.iso_id)
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unmount(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let (integration_id, server) = server_context(&state, &user, id).await?;
    store(&state, integration_id)
        .await?
        .unmount(&server)
        .await
        .map_err(|error| AppError::field("iso_id", format!("{error:#}")))?;
    sqlx::query(
        "UPDATE vps_isos SET mounted_on = NULL, last_used_at = now() WHERE mounted_on = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn remove_unused(state: &AppState) -> AppResult<usize> {
    let stale: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM vps_isos
         WHERE mounted_on IS NULL AND status <> 'importing' AND last_used_at < now() - make_interval(days => $1)",
    )
    .bind(UNUSED_DAYS)
    .fetch_all(&state.db)
    .await?;
    let mut removed = 0;
    for id in stale {
        match remove(state, id).await {
            Ok(()) => removed += 1,
            Err(error) => {
                tracing::warn!(%id, error = %format!("{error:#}"), "removing an unused ISO failed")
            }
        }
    }
    Ok(removed)
}
