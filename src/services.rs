use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::integrations::{self, proxmox, pterodactyl, webhosting};
use crate::state::AppState;

#[derive(sqlx::FromRow)]
struct ProvisionContext {
    name: String,
    kind: String,
    integration_id: Option<Uuid>,
    provisioning: Value,
    details: Value,
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

pub async fn recover_interrupted(state: &AppState) -> anyhow::Result<()> {
    let services = sqlx::query(
        "UPDATE services SET status = 'failed',
                last_error = 'Stacklane werd herstart tijdens het leveren. Probeer het opnieuw.', updated_at = now()
         WHERE status = 'provisioning'",
    )
    .execute(&state.db)
    .await?
    .rows_affected();
    if services > 0 {
        tracing::warn!(
            services,
            "marked deliveries interrupted by a restart as failed"
        );
    }
    Ok(())
}

async fn claim(state: &AppState, service_id: Uuid) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar::<_, Uuid>(
        "UPDATE services SET status = 'provisioning', last_error = NULL, updated_at = now()
         WHERE id = $1 AND status IN ('pending', 'failed') RETURNING id",
    )
    .bind(service_id)
    .fetch_optional(&state.db)
    .await?
    .is_some())
}

async fn run_provision(state: &AppState, service_id: Uuid) -> anyhow::Result<()> {
    let context: ProvisionContext = sqlx::query_as(
        "SELECT s.name, s.kind, s.integration_id, p.provisioning, s.details, u.email::text AS email, u.first_name, u.last_name, u.id AS user_id
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
        "vps" => {
            let client = driver
                .proxmox()
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            provision_vps(state, service_id, integration_id, client, &context).await?
        }
        "webhosting" => {
            let hosting = webhosting::Hosting::from_driver(&driver)
                .ok_or_else(|| anyhow::anyhow!("deze koppeling kan geen webhosting maken"))?;
            provision_webhosting(service_id, &hosting, &context).await?
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

#[derive(Deserialize, Default)]
struct VpsOrder {
    #[serde(default)]
    image: Option<u32>,
    #[serde(default)]
    ssh_key_ids: Vec<Uuid>,
    #[serde(default)]
    vmid: Option<u32>,
}

fn hostname(name: &str) -> String {
    let cleaned: String = name
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "vps".to_owned()
    } else {
        trimmed.chars().take(63).collect()
    }
}

fn created_for(description: &str, service_id: Uuid) -> bool {
    description
        .strip_prefix("Stacklane ")
        .and_then(|rest| rest.split_whitespace().next())
        == Some(service_id.to_string().as_str())
}

fn root_password() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    rand::random::<[u8; 20]>()
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(*byte) % ALPHABET.len()]))
        .collect()
}

async fn provision_vps(
    state: &AppState,
    service_id: Uuid,
    integration_id: Uuid,
    client: &proxmox::Proxmox,
    context: &ProvisionContext,
) -> anyhow::Result<pterodactyl::Provisioned> {
    let config: proxmox::ProductConfig = serde_json::from_value(context.provisioning.clone())?;
    let order: VpsOrder = serde_json::from_value(context.details.clone()).unwrap_or_default();
    let settings = client.settings();
    let image = order
        .image
        .and_then(|template| {
            settings
                .images
                .iter()
                .find(|image| image.template == template)
        })
        .or_else(|| settings.images.first())
        .ok_or_else(|| anyhow::anyhow!("er staat geen besturingssysteem op deze koppeling"))?
        .clone();

    let mut transaction = state.db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text))")
        .bind(integration_id)
        .execute(&mut *transaction)
        .await?;
    let recorded: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM services, LATERAL (VALUES (external_id), (details->>'vmid')) AS ids(id)
         WHERE integration_id = $1 AND services.id <> $2 AND kind = 'vps'
           AND status <> 'terminated' AND ids.id IS NOT NULL",
    )
    .bind(integration_id)
    .bind(service_id)
    .fetch_all(&mut *transaction)
    .await?;
    let recorded: Vec<u32> = recorded.iter().filter_map(|id| id.parse().ok()).collect();
    let mut stale = None;
    if let Some(previous) = order.vmid
        && !recorded.contains(&previous)
        && client
            .description(previous)
            .await
            .is_some_and(|description| created_for(&description, service_id))
    {
        stale = Some(previous);
    }
    let taken: Vec<String> = sqlx::query_scalar(
        "SELECT details->>'ipv4' FROM services
         WHERE integration_id = $1 AND id <> $2 AND status <> 'terminated' AND details ? 'ipv4'",
    )
    .bind(integration_id)
    .bind(service_id)
    .fetch_all(&mut *transaction)
    .await?;
    let ipv4 = proxmox::ip_range(settings)
        .find(|ip| !taken.contains(&ip.to_string()))
        .ok_or_else(|| anyhow::anyhow!("alle IP-adressen van deze koppeling zijn in gebruik"))?;
    if let Some(previous) = stale {
        client.destroy(previous).await?;
    }
    let vmid = client.free_vmid(&recorded).await?;
    sqlx::query("UPDATE services SET details = details || $2 WHERE id = $1")
        .bind(service_id)
        .bind(json!({ "ipv4": ipv4.to_string(), "vmid": vmid }))
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;

    let ssh_keys: Vec<String> =
        sqlx::query_scalar("SELECT public_key FROM ssh_keys WHERE user_id = $1 AND id = ANY($2)")
            .bind(context.user_id)
            .bind(&order.ssh_key_ids)
            .fetch_all(&state.db)
            .await?;
    let password = root_password();
    let hostname = hostname(&context.name);
    client
        .create(
            &proxmox::NewServer {
                vmid,
                hostname: &hostname,
                template: image.template,
                ipv4,
                ssh_keys: &ssh_keys,
                password: &password,
                description: &format!("Stacklane {service_id} ({})", context.email),
            },
            &config,
        )
        .await?;

    let mut details = context.details.clone();
    let extra = json!({
        "node": settings.node,
        "vmid": vmid,
        "ipv4": ipv4.to_string(),
        "hostname": hostname,
        "image": image.name,
        "cores": config.cores,
        "memory_mb": config.memory_mb,
        "disk_gb": config.disk_gb,
        "root_password": crate::payments::conceal(state, &password).map_err(|error| anyhow::anyhow!("{error}"))?,
    });
    if let (Some(target), Some(source)) = (details.as_object_mut(), extra.as_object()) {
        target.extend(source.clone());
    }
    Ok(pterodactyl::Provisioned {
        external_id: vmid.to_string(),
        details,
    })
}

async fn provision_webhosting(
    service_id: Uuid,
    hosting: &webhosting::Hosting<'_>,
    context: &ProvisionContext,
) -> anyhow::Result<pterodactyl::Provisioned> {
    let plan = context.provisioning["plan"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("het product heeft geen hostingpakket"))?;
    let suffix: String = service_id.simple().to_string().chars().take(2).collect();
    let username = webhosting::username_for(&context.name, &suffix);
    let password = webhosting::password();
    let owner = format!("{} {}", context.first_name, context.last_name);
    let account = webhosting::NewAccount {
        username: &username,
        domain: &context.name,
        email: &context.email,
        password: &password,
        plan,
        owner: owner.trim(),
    };
    let external_id = hosting.create(&account).await?;
    Ok(pterodactyl::Provisioned {
        external_id,
        details: json!({
            "username": username,
            "plan": plan,
            "panel": hosting.panel_name(),
            "panel_url": hosting.panel_url(),
        }),
    })
}

pub fn provision_in_background(state: AppState, service_id: Uuid) {
    tokio::spawn(async move {
        match claim(&state, service_id).await {
            Ok(true) => {}
            Ok(false) => return,
            Err(error) => {
                tracing::warn!(%service_id, error = %format!("{error:#}"), "claiming a service failed");
                return;
            }
        }
        if let Err(error) = run_provision(&state, service_id).await {
            mark_failed(&state, service_id, &error).await;
        }
    });
}

pub async fn change_state(state: &AppState, service_id: Uuid, action: &str) -> AppResult<()> {
    change_state_because(state, service_id, action, "admin").await
}

pub async fn change_state_because(
    state: &AppState,
    service_id: Uuid,
    action: &str,
    reason: &str,
) -> AppResult<()> {
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

    sqlx::query(
        "UPDATE services SET status = $2, updated_at = now(),
                suspension_reason = CASE WHEN $2 = 'suspended' THEN $3 END
         WHERE id = $1",
    )
    .bind(service_id)
    .bind(next)
    .bind(reason)
    .execute(&state.db)
    .await?;
    if action == "terminate" {
        end_billing(state, service_id).await?;
    }
    Ok(())
}

async fn end_billing(state: &AppState, service_id: Uuid) -> AppResult<()> {
    let subscription: Option<Uuid> =
        sqlx::query_scalar("SELECT subscription_id FROM services WHERE id = $1")
            .bind(service_id)
            .fetch_one(&state.db)
            .await?;
    let Some(subscription) = subscription else {
        return Ok(());
    };
    let others: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM services WHERE subscription_id = $1 AND id <> $2 AND status <> 'terminated')",
    )
    .bind(subscription)
    .bind(service_id)
    .fetch_one(&state.db)
    .await?;
    if others {
        return Ok(());
    }
    let mut transaction = state.db.begin().await?;
    sqlx::query(
        "UPDATE subscriptions SET status = 'cancelled', cancelled_at = COALESCE(cancelled_at, now())
         WHERE id = $1 AND status IN ('pending', 'active', 'suspended')",
    )
    .bind(subscription)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE invoices v SET status = 'cancelled'
         WHERE v.status = 'open'
           AND EXISTS (SELECT 1 FROM invoice_lines l WHERE l.invoice_id = v.id)
           AND NOT EXISTS (SELECT 1 FROM invoice_lines l WHERE l.invoice_id = v.id
                           AND l.subscription_id IS DISTINCT FROM $1)",
    )
    .bind(subscription)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_claims_its_own_vm() {
        let service = Uuid::from_u128(1);
        let other = Uuid::from_u128(2);
        assert!(created_for(
            &format!("Stacklane {service} (jan@example.test)"),
            service
        ));
        assert!(!created_for(
            &format!("Stacklane {other} (piet@example.test)"),
            service
        ));
        assert!(!created_for(&format!("Klant {service}"), service));
        assert!(!created_for("", service));
    }
}
