use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::activity;
use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::limits::ClientIp;
use crate::modrinth::{self, Loader};
use crate::routes::game_access;
use crate::routes::gameservers::panel;
use crate::state::AppState;

#[derive(Serialize)]
pub struct Context {
    loader: Option<&'static str>,
    version: Option<String>,
    loaders: Vec<&'static str>,
}

#[derive(Deserialize)]
pub struct LoaderQuery {
    loader: String,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    loader: String,
    #[serde(default)]
    query: String,
    version: Option<String>,
}

#[derive(Deserialize)]
pub struct Install {
    project: String,
    loader: String,
    version: Option<String>,
}

#[derive(Deserialize)]
pub struct Removal {
    loader: String,
    file: String,
}

fn loader(value: &str) -> AppResult<Loader> {
    Loader::parse(value)
        .ok_or_else(|| AppError::field("loader", "Kies een servertype uit de lijst."))
}

fn version(value: Option<&str>) -> AppResult<Option<String>> {
    match value.map(str::trim) {
        None | Some("") => Ok(None),
        Some(value) => modrinth::minecraft_version(value).map(Some).ok_or_else(|| {
            AppError::field("version", "Gebruik een Minecraft-versie zoals 1.21.1.")
        }),
    }
}

fn jar(value: &str) -> AppResult<&str> {
    let value = value.trim();
    let valid = value.ends_with(".jar")
        && !value.starts_with('.')
        && value.len() <= 200
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+' | ' '));
    if valid {
        Ok(value)
    } else {
        Err(AppError::field(
            "file",
            "Dit is geen plugin- of modbestand.",
        ))
    }
}

async fn allowed(state: &AppState, user: &CurrentUser, id: Uuid) -> AppResult<()> {
    state
        .limits
        .check(&crate::limits::GAME_PANEL, &user.id.to_string())?;
    game_access::grant(state, user.id, id)
        .await?
        .require("files")
}

fn upstream(error: anyhow::Error) -> AppError {
    AppError::upstream("server", error)
}

pub async fn context(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Context>> {
    allowed(&state, &user, id).await?;
    let egg: Option<String> =
        sqlx::query_scalar("SELECT details->>'egg' FROM services WHERE id = $1")
            .bind(id)
            .fetch_one(&state.db)
            .await?;
    let variables = match panel(&state, &user, id, "files").await {
        Ok((client, identifier)) => client
            .startup_variables(&identifier)
            .await
            .unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let version = [
        "MINECRAFT_VERSION",
        "MC_VERSION",
        "VERSION",
        "VANILLA_VERSION",
    ]
    .into_iter()
    .find_map(|name| {
        variables
            .iter()
            .find(|(key, _)| key == name)
            .and_then(|(_, value)| modrinth::minecraft_version(value))
    });
    Ok(Json(Context {
        loader: egg.as_deref().and_then(Loader::from_egg).map(Loader::key),
        version,
        loaders: Loader::ALL.into_iter().map(Loader::key).collect(),
    }))
}

pub async fn installed(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Query(query): Query<LoaderQuery>,
) -> AppResult<Json<Value>> {
    let loader = loader(&query.loader)?;
    let (client, identifier) = panel(&state, &user, id, "files").await?;
    let entries = match client.list_files(&identifier, loader.folder()).await {
        Ok(entries) => entries,
        Err(_) => Value::Array(Vec::new()),
    };
    let files: Vec<Value> = entries
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            entry["is_file"].as_bool().unwrap_or(false)
                && entry["name"].as_str().is_some_and(|name| name.ends_with(".jar"))
        })
        .map(|entry| json!({ "name": entry["name"], "size": entry["size"], "modified_at": entry["modified_at"] }))
        .collect();
    Ok(Json(json!({ "folder": loader.folder(), "files": files })))
}

pub async fn search(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<modrinth::Hit>>> {
    let loader = loader(&query.loader)?;
    let version = version(query.version.as_deref())?;
    allowed(&state, &user, id).await?;
    let text: String = query.query.trim().chars().take(100).collect();
    let hits = modrinth::search(
        &state.config.modrinth_url,
        &text,
        loader,
        version.as_deref(),
    )
    .await
    .map_err(|error| AppError::upstream("query", error))?;
    Ok(Json(hits))
}

pub async fn install(
    State(state): State<AppState>,
    user: CurrentUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<Uuid>,
    Json(input): Json<Install>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let loader = loader(&input.loader)?;
    let version = version(input.version.as_deref())?;
    let project = modrinth::project_id(&input.project)
        .ok_or_else(|| AppError::field("project", "Onbekende plugin."))?;
    let (client, identifier) = panel(&state, &user, id, "files").await?;
    let (title, download) = modrinth::resolve(
        &state.config.modrinth_url,
        project,
        loader,
        version.as_deref(),
    )
    .await
    .map_err(|error| AppError::upstream("project", error))?
    .ok_or_else(|| {
        AppError::field(
            "project",
            "Er is geen versie die past bij dit servertype en deze Minecraft-versie.",
        )
    })?;
    client
        .pull_file(
            &identifier,
            &download.url,
            loader.folder(),
            &download.filename,
        )
        .await
        .map_err(upstream)?;
    activity::record(
        &state,
        id,
        user.id,
        &ip,
        "addon.install",
        Some(&format!("{title} ({})", download.filename)),
    )
    .await;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "title": title, "file": download.filename, "folder": loader.folder() })),
    ))
}

pub async fn remove(
    State(state): State<AppState>,
    user: CurrentUser,
    ClientIp(ip): ClientIp,
    Path(id): Path<Uuid>,
    Query(query): Query<Removal>,
) -> AppResult<StatusCode> {
    let loader = loader(&query.loader)?;
    let file = jar(&query.file)?;
    let (client, identifier) = panel(&state, &user, id, "files").await?;
    client
        .delete_files(&identifier, loader.folder(), &[file])
        .await
        .map_err(upstream)?;
    activity::record(&state, id, user.id, &ip, "addon.remove", Some(file)).await;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::jar;

    #[test]
    fn only_jar_files_in_the_folder_can_be_removed() {
        assert!(jar("EssentialsX-2.20.1.jar").is_ok());
        assert!(jar("../server.properties").is_err());
        assert!(jar("plugins/x.jar").is_err());
        assert!(jar(".hidden.jar").is_err());
        assert!(jar("world").is_err());
    }
}
