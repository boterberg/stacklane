use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::vps::machine_of;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, isos};
use crate::routes::vps_access::VpsActor;
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
    #[serde(serialize_with = "crate::error::public_error")]
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

pub async fn index(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Overview>> {
    actor.require("isos.view")?;
    let isos: Vec<Iso> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} ORDER BY i.created_at"
    )))
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(Overview {
        used: i64::try_from(isos.len()).unwrap_or(i64::MAX),
        isos,
        limit: MAX_ISOS,
        can_import: capable_server(&state, actor.owner).await?.is_some(),
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
    actor: VpsActor,
    Json(input): Json<NewIso>,
) -> AppResult<(StatusCode, Json<Iso>)> {
    actor.require("isos.import")?;
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
        .map_err(|error| AppError::field("url", error.to_string()))?;
    let mut transaction = state.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text))")
        .bind(format!("isos {}", actor.owner))
        .execute(&mut *transaction)
        .await?;
    let used: i64 = sqlx::query_scalar("SELECT count(*) FROM vps_isos WHERE user_id = $1")
        .bind(actor.owner)
        .fetch_one(&mut *transaction)
        .await?;
    if used >= MAX_ISOS {
        return Err(AppError::field(
            "name",
            "Je hebt al 2 ISO's. Verwijder er eerst een.",
        ));
    }
    let Some(server) = capable_server(&state, actor.owner).await? else {
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
    .bind(actor.owner)
    .bind(integration_id)
    .bind(name)
    .bind(&url)
    .bind(&input.firmware)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    tokio::spawn(run_import(
        state.clone(),
        id,
        integration_id,
        name.to_owned(),
        url,
        input.firmware == "uefi",
    ));
    let iso = sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND i.id = $2")))
        .bind(actor.owner)
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
    let store = store(state, integration_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    match external_id {
        Some(external_id) => store.delete(&external_id).await?,
        None => {
            if let isos::IsoStore::Proxmox(_) = &store {
                store
                    .delete(&format!("stacklane-{}.iso", id.simple()))
                    .await
                    .ok();
            }
        }
    }
    sqlx::query("DELETE FROM vps_isos WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(())
}

pub async fn destroy(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    actor.require("isos.delete")?;
    let row: Option<(String, Option<Uuid>)> =
        sqlx::query_as("SELECT status, mounted_on FROM vps_isos WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(actor.owner)
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
        .map_err(|error| AppError::upstream("iso", error))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn server_context(state: &AppState, actor: &VpsActor, id: Uuid) -> AppResult<(Uuid, String)> {
    machine_of(state, actor.owner, id).await?;
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
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ServerIso>> {
    actor.require("isos.view")?;
    let integration_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT integration_id FROM services WHERE id = $1 AND user_id = $2 AND kind = 'vps'",
    )
    .bind(id)
    .bind(actor.owner)
    .fetch_optional(&state.db)
    .await?
    .flatten();
    let Some(integration_id) = integration_id else {
        return Err(AppError::NotFound);
    };
    let isos: Vec<Iso> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} AND i.integration_id = $2 ORDER BY i.created_at"
    )))
    .bind(actor.owner)
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
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<Mount>,
) -> AppResult<StatusCode> {
    actor.require("isos.mount")?;
    let (integration_id, server) = server_context(&state, &actor, id).await?;
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
    let rescued: bool = sqlx::query_scalar("SELECT details ? 'rescue' FROM services WHERE id = $1")
        .bind(id)
        .fetch_one(&state.db)
        .await?;
    if rescued {
        return Err(AppError::field("iso_id", "Zet eerst de rescue-modus uit."));
    }
    let iso: Option<(String, String)> = sqlx::query_as(
        "UPDATE vps_isos SET mounted_on = $4, last_used_at = now()
         WHERE id = $1 AND user_id = $2 AND integration_id = $3 AND status = 'ready' AND mounted_on IS NULL
         RETURNING external_id, firmware",
    )
    .bind(input.iso_id)
    .bind(actor.owner)
    .bind(integration_id)
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| match error {
        sqlx::Error::Database(ref database) if database.is_unique_violation() => {
            AppError::field("iso_id", "Koppel eerst de huidige ISO los.")
        }
        error => error.into(),
    })?;
    let Some((external_id, firmware)) = iso else {
        return Err(AppError::field("iso_id", "Kies een ISO die klaar is."));
    };
    let mounted = async {
        store(&state, integration_id)
            .await?
            .mount(&server, &external_id, firmware == "uefi")
            .await
            .map_err(|error| AppError::upstream("iso_id", error))
    }
    .await;
    if let Err(error) = mounted {
        sqlx::query("UPDATE vps_isos SET mounted_on = NULL WHERE id = $1 AND mounted_on = $2")
            .bind(input.iso_id)
            .bind(id)
            .execute(&state.db)
            .await?;
        return Err(error);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unmount(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    actor.require("isos.mount")?;
    let (integration_id, server) = server_context(&state, &actor, id).await?;
    store(&state, integration_id)
        .await?
        .unmount(&server)
        .await
        .map_err(|error| AppError::upstream("iso_id", error))?;
    sqlx::query(
        "UPDATE vps_isos SET mounted_on = NULL, last_used_at = now() WHERE mounted_on = $1",
    )
    .bind(id)
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn remove_orphans(state: &AppState) -> AppResult<()> {
    let integrations: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM integrations WHERE kind = 'proxmox'")
            .fetch_all(&state.db)
            .await?;
    for integration_id in integrations {
        let Ok(isos::IsoStore::Proxmox(client)) = store(state, integration_id).await else {
            continue;
        };
        let Ok(files) = client.stacklane_isos().await else {
            continue;
        };
        let known: Vec<String> = sqlx::query_scalar(
            "SELECT 'stacklane-' || replace(id::text, '-', '') || '.iso' FROM vps_isos
             WHERE status <> 'failed' OR created_at > now() - interval '2 hours'",
        )
        .fetch_all(&state.db)
        .await?;
        for file in files.iter().filter(|file| !known.contains(file)) {
            client.delete_iso(file).await.ok();
        }
    }
    Ok(())
}

pub async fn remove_unused(state: &AppState) -> AppResult<usize> {
    if let Err(error) = remove_orphans(state).await {
        tracing::warn!(%error, "removing orphaned ISO files failed");
    }
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
