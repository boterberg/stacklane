use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{Staff, permission};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Extension {
    id: Uuid,
    integration_id: Uuid,
    integration_name: String,
    extension: String,
    register_cents: i32,
    renew_cents: i32,
    transfer_cents: i32,
    is_active: bool,
    domains: i64,
}

#[derive(Deserialize)]
pub struct ExtensionInput {
    integration_id: Uuid,
    extension: String,
    register_cents: i32,
    renew_cents: i32,
    transfer_cents: i32,
    #[serde(default = "active")]
    is_active: bool,
}

fn active() -> bool {
    true
}

const SELECT: &str = "SELECT e.id, e.integration_id, i.name AS integration_name, e.extension, e.register_cents,
        e.renew_cents, e.transfer_cents, e.is_active,
        (SELECT count(*) FROM services s WHERE s.kind = 'domain' AND s.status NOT IN ('terminated', 'failed')
            AND s.name LIKE '%.' || e.extension) AS domains
     FROM domain_extensions e JOIN integrations i ON i.id = e.integration_id";

pub fn normalize_extension(extension: &str) -> Option<String> {
    let extension = extension.trim().trim_start_matches('.').to_lowercase();
    let valid = !extension.is_empty()
        && extension.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '-')
        });
    valid.then_some(extension)
}

async fn validate(state: &AppState, input: &ExtensionInput) -> AppResult<String> {
    let mut errors = BTreeMap::new();
    let extension = normalize_extension(&input.extension);
    if extension.is_none() {
        errors.insert(
            "extension",
            "Vul een extensie in, zoals nl of co.uk.".to_owned(),
        );
    }
    for (field, value) in [
        ("register_cents", input.register_cents),
        ("renew_cents", input.renew_cents),
        ("transfer_cents", input.transfer_cents),
    ] {
        if !(0..=10_000_000).contains(&value) {
            errors.insert(field, "Vul een prijs van 0 of hoger in.".to_owned());
        }
    }
    let kind: Option<String> = sqlx::query_scalar("SELECT kind FROM integrations WHERE id = $1")
        .bind(input.integration_id)
        .fetch_optional(&state.db)
        .await?;
    if kind.as_deref() != Some("openprovider") {
        errors.insert(
            "integration_id",
            "Kies een koppeling met een registrar.".to_owned(),
        );
    }
    match extension {
        Some(extension) if errors.is_empty() => Ok(extension),
        _ => Err(AppError::Validation(errors)),
    }
}

pub async fn index(
    State(state): State<AppState>,
    _admin: Staff<permission::Services>,
) -> AppResult<Json<Vec<Extension>>> {
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{SELECT} ORDER BY e.extension"
        )))
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    _admin: Staff<permission::Services>,
    Json(input): Json<ExtensionInput>,
) -> AppResult<StatusCode> {
    let extension = validate(&state, &input).await?;
    let inserted = sqlx::query(
        "INSERT INTO domain_extensions (id, integration_id, extension, register_cents, renew_cents, transfer_cents, is_active)
         VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (extension) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(input.integration_id)
    .bind(&extension)
    .bind(input.register_cents)
    .bind(input.renew_cents)
    .bind(input.transfer_cents)
    .bind(input.is_active)
    .execute(&state.db)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(AppError::field("extension", "Deze extensie staat er al."));
    }
    Ok(StatusCode::CREATED)
}

pub async fn update(
    State(state): State<AppState>,
    _admin: Staff<permission::Services>,
    Path(id): Path<Uuid>,
    Json(input): Json<ExtensionInput>,
) -> AppResult<StatusCode> {
    let extension = validate(&state, &input).await?;
    let updated = sqlx::query(
        "UPDATE domain_extensions SET integration_id = $2, extension = $3, register_cents = $4, renew_cents = $5,
            transfer_cents = $6, is_active = $7
         WHERE id = $1",
    )
    .bind(id)
    .bind(input.integration_id)
    .bind(&extension)
    .bind(input.register_cents)
    .bind(input.renew_cents)
    .bind(input.transfer_cents)
    .bind(input.is_active)
    .execute(&state.db)
    .await
    .map_err(|error| match &error {
        sqlx::Error::Database(database) if database.is_unique_violation() => {
            AppError::field("extension", "Deze extensie staat er al.")
        }
        _ => AppError::from(error),
    })?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn destroy(
    State(state): State<AppState>,
    _admin: Staff<permission::Services>,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let deleted = sqlx::query("DELETE FROM domain_extensions WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::normalize_extension;

    #[test]
    fn normalizes_extensions() {
        assert_eq!(normalize_extension(" .NL "), Some("nl".to_owned()));
        assert_eq!(normalize_extension("co.uk"), Some("co.uk".to_owned()));
        assert_eq!(normalize_extension("..nl"), Some("nl".to_owned()));
        assert_eq!(normalize_extension("n l"), None);
        assert_eq!(normalize_extension(""), None);
    }
}
