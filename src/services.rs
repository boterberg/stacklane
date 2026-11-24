use serde_json::Value;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::integrations::{self, pterodactyl};
use crate::state::AppState;

#[derive(sqlx::FromRow)]
struct ProvisionContext {
    name: String,
    kind: String,
    integration_id: Option<Uuid>,
    provisioning: Value,
    email: String,
    first_name: String,
    last_name: String,
    user_id: Uuid,
}

async fn mark_failed(state: &AppState, service_id: Uuid, error: &anyhow::Error) {
    tracing::warn!(%service_id, error = %format!("{error:#}"), "provisioning failed");
    let _ = sqlx::query(
        "UPDATE services SET status = 'failed', last_error = $2, updated_at = now() WHERE id = $1",
    )
    .bind(service_id)
    .bind(format!("{error:#}"))
    .execute(&state.db)
    .await;
}

async fn run_provision(state: &AppState, service_id: Uuid) -> anyhow::Result<()> {
    let context: ProvisionContext = sqlx::query_as(
        "SELECT s.name, s.kind, s.integration_id, p.provisioning, u.email::text AS email, u.first_name, u.last_name, u.id AS user_id
         FROM services s
         JOIN users u ON u.id = s.user_id
         JOIN products p ON p.id = s.product_id
         WHERE s.id = $1",
    )
    .bind(service_id)
    .fetch_one(&state.db)
    .await?;
    let integration_id = context
        .integration_id
        .ok_or_else(|| anyhow::anyhow!("het product is niet aan een koppeling gekoppeld"))?;

    sqlx::query("UPDATE services SET status = 'provisioning', last_error = NULL, updated_at = now() WHERE id = $1")
        .bind(service_id)
        .execute(&state.db)
        .await?;

    let driver = integrations::driver(state, integration_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let provisioned = match context.kind.as_str() {
        "gameserver" => {
            let config: pterodactyl::ProductConfig = serde_json::from_value(context.provisioning)?;
            let owner = pterodactyl::Owner {
                email: &context.email,
                first_name: &context.first_name,
                last_name: &context.last_name,
                stacklane_id: &context.user_id.to_string(),
            };
            driver
                .pterodactyl()
                .map_err(|error| anyhow::anyhow!("{error}"))?
                .provision(&service_id.to_string(), &context.name, &owner, &config)
                .await?
        }
        other => anyhow::bail!(
            "diensten van het type {other} kunnen nog niet automatisch worden aangemaakt"
        ),
    };

    sqlx::query(
        "UPDATE services SET status = 'active', external_id = $2, details = $3, last_error = NULL, updated_at = now() WHERE id = $1",
    )
    .bind(service_id)
    .bind(provisioned.external_id)
    .bind(provisioned.details)
    .execute(&state.db)
    .await?;
    Ok(())
}

pub fn provision_in_background(state: AppState, service_id: Uuid) {
    tokio::spawn(async move {
        if let Err(error) = run_provision(&state, service_id).await {
            mark_failed(&state, service_id, &error).await;
        }
    });
}

pub async fn change_state(state: &AppState, service_id: Uuid, action: &str) -> AppResult<()> {
    let (integration_id, external_id, status): (Option<Uuid>, Option<String>, String) =
        sqlx::query_as("SELECT integration_id, external_id, status FROM services WHERE id = $1")
            .bind(service_id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;

    let next = match (action, status.as_str()) {
        ("suspend", "active") => "suspended",
        ("unsuspend", "suspended") => "active",
        ("terminate", "active" | "suspended" | "failed" | "pending") => "terminated",
        _ => {
            return Err(AppError::field(
                "action",
                "Deze actie kan niet in de huidige status.",
            ));
        }
    };

    if let (Some(integration_id), Some(external_id)) = (integration_id, external_id.as_deref()) {
        let driver = integrations::driver(state, integration_id).await?;
        let result = match action {
            "suspend" => driver.suspend(external_id).await,
            "unsuspend" => driver.unsuspend(external_id).await,
            _ => driver.terminate(external_id).await,
        };
        result.map_err(|error| AppError::field("action", format!("{error:#}")))?;
    }

    sqlx::query("UPDATE services SET status = $2, updated_at = now() WHERE id = $1")
        .bind(service_id)
        .bind(next)
        .execute(&state.db)
        .await?;
    Ok(())
}
