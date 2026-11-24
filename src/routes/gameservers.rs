use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, pterodactyl::Pterodactyl};
use crate::state::AppState;

const MAX_EDIT_SIZE: usize = 2 * 1024 * 1024;

#[derive(Serialize, sqlx::FromRow)]
pub struct GameServer {
    id: Uuid,
    name: String,
    status: String,
    details: Value,
    last_error: Option<String>,
    product_name: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct Access {
    status: String,
    integration_id: Option<Uuid>,
    identifier: Option<String>,
}

#[derive(Deserialize)]
pub struct PowerRequest {
    signal: String,
}

#[derive(Deserialize)]
pub struct DirectoryQuery {
    directory: Option<String>,
}

#[derive(Deserialize)]
pub struct FileQuery {
    file: String,
}

#[derive(Deserialize)]
pub struct BackupRequest {
    name: Option<String>,
}

const SELECT: &str =
    "SELECT s.id, s.name, s.status, s.details, s.last_error, p.name AS product_name, s.created_at
    FROM services s LEFT JOIN products p ON p.id = s.product_id";

fn clean_path(path: &str) -> AppResult<String> {
    if path.split('/').any(|segment| segment == "..") {
        return Err(AppError::field("path", "Ongeldig pad."));
    }
    let trimmed = path.trim();
    Ok(if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    })
}

async fn panel(state: &AppState, user: &CurrentUser, id: Uuid) -> AppResult<(Pterodactyl, String)> {
    let access: Access = sqlx::query_as(
        "SELECT status, integration_id, details->>'identifier' AS identifier FROM services
         WHERE id = $1 AND kind = 'gameserver' AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    match access.status.as_str() {
        "active" => {}
        "suspended" => {
            return Err(AppError::field(
                "status",
                "Deze server is opgeschort. Neem contact op met support.",
            ));
        }
        _ => return Err(AppError::field("status", "Deze server is nog niet klaar.")),
    }
    let integration_id = access.integration_id.ok_or(AppError::NotFound)?;
    let identifier = access.identifier.ok_or(AppError::NotFound)?;
    let client = integrations::driver(state, integration_id)
        .await?
        .into_pterodactyl()?;
    Ok((client, identifier))
}

fn upstream(error: anyhow::Error) -> AppError {
    AppError::field("server", format!("{error:#}"))
}

pub async fn index(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<Vec<GameServer>>> {
    let rows = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE s.user_id = $1 AND s.kind = 'gameserver' AND s.status <> 'terminated' ORDER BY s.created_at"
    )))
    .bind(user.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub async fn show(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let server: GameServer = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE s.id = $1 AND s.user_id = $2 AND s.kind = 'gameserver' AND s.status <> 'terminated'"
    )))
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    let mut install_state = None;
    let mut resources = None;
    if server.status == "active" {
        let access: Option<(Option<Uuid>, Option<String>)> =
            sqlx::query_as("SELECT integration_id, external_id FROM services WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.db)
                .await?;
        if let Some((Some(integration_id), Some(external_id))) = access {
            let client = integrations::driver(&state, integration_id)
                .await?
                .into_pterodactyl()?;
            install_state = client.install_state(&external_id).await.ok().flatten();
            if install_state.is_none()
                && let Some(identifier) = server.details["identifier"].as_str()
            {
                resources = client.resources(identifier).await.ok();
            }
        }
    }
    let mut value = serde_json::to_value(server).map_err(anyhow::Error::from)?;
    value["resources"] = resources.unwrap_or(Value::Null);
    value["install_state"] = install_state.map(Value::String).unwrap_or(Value::Null);
    Ok(Json(value))
}

pub async fn power(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<PowerRequest>,
) -> AppResult<StatusCode> {
    if !matches!(input.signal.as_str(), "start" | "stop" | "restart" | "kill") {
        return Err(AppError::field("signal", "Onbekende actie."));
    }
    let (client, identifier) = panel(&state, &user, id).await?;
    client
        .power(&identifier, &input.signal)
        .await
        .map_err(upstream)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn websocket(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let (client, identifier) = panel(&state, &user, id).await?;
    Ok(Json(client.websocket(&identifier).await.map_err(upstream)?))
}

pub async fn files(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Query(query): Query<DirectoryQuery>,
) -> AppResult<Json<Value>> {
    let directory = clean_path(query.directory.as_deref().unwrap_or("/"))?;
    let (client, identifier) = panel(&state, &user, id).await?;
    Ok(Json(json!({
        "directory": directory,
        "entries": client.list_files(&identifier, &directory).await.map_err(upstream)?,
    })))
}

pub async fn read_file(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Query(query): Query<FileQuery>,
) -> AppResult<Json<Value>> {
    let file = clean_path(&query.file)?;
    let (client, identifier) = panel(&state, &user, id).await?;
    let content = client
        .read_file(&identifier, &file)
        .await
        .map_err(upstream)?;
    Ok(Json(json!({ "file": file, "content": content })))
}

pub async fn write_file(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Query(query): Query<FileQuery>,
    body: String,
) -> AppResult<StatusCode> {
    if body.len() > MAX_EDIT_SIZE {
        return Err(AppError::field(
            "content",
            "Dit bestand is te groot om hier te bewerken.",
        ));
    }
    let file = clean_path(&query.file)?;
    let (client, identifier) = panel(&state, &user, id).await?;
    client
        .write_file(&identifier, &file, body)
        .await
        .map_err(upstream)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn backups(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let (client, identifier) = panel(&state, &user, id).await?;
    Ok(Json(client.backups(&identifier).await.map_err(upstream)?))
}

pub async fn create_backup(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<BackupRequest>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let name = input
        .name
        .map(|name| name.trim().chars().take(60).collect::<String>())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "Back-up".to_owned());
    let (client, identifier) = panel(&state, &user, id).await?;
    let backup = client
        .create_backup(&identifier, &name)
        .await
        .map_err(upstream)?;
    Ok((StatusCode::CREATED, Json(backup)))
}

pub async fn download_backup(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((id, backup)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<Value>> {
    let (client, identifier) = panel(&state, &user, id).await?;
    let url = client
        .backup_download(&identifier, &backup.to_string())
        .await
        .map_err(upstream)?;
    Ok(Json(json!({ "url": url })))
}

#[cfg(test)]
mod tests {
    use super::clean_path;

    #[test]
    fn paths_cannot_escape_the_server_root() {
        assert!(clean_path("../etc/passwd").is_err());
        assert!(clean_path("/plugins/../../x").is_err());
        assert_eq!(
            clean_path("server.properties").unwrap(),
            "/server.properties"
        );
        assert_eq!(clean_path("/world").unwrap(), "/world");
    }
}
