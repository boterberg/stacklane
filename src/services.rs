use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::integrations::{
    self, openprovider, openstack, proxmox, pterodactyl, virtfusion, webhosting,
};
use crate::state::AppState;

#[derive(sqlx::FromRow)]
struct ProvisionContext {
    name: String,
    kind: String,
    integration_id: Option<Uuid>,
    provisioning: Value,
    details: Value,
    email: String,
    email_verified: bool,
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
    let isos = sqlx::query(
        "UPDATE vps_isos SET status = 'failed', last_error = 'Stacklane werd herstart tijdens het importeren.'
         WHERE status = 'importing'",
    )
    .execute(&state.db)
    .await?
    .rows_affected();
    let rebuilds = sqlx::query(
        "UPDATE services SET details = details - 'rebuilding' - 'rebuild_scratch',
                last_error = $1, updated_at = now()
         WHERE details ? 'rebuilding'",
    )
    .bind(REBUILD_INTERRUPTED)
    .execute(&state.db)
    .await?
    .rows_affected();
    if services + isos + rebuilds > 0 {
        tracing::warn!(
            services,
            isos,
            rebuilds,
            "marked work interrupted by a restart as failed"
        );
    }
    Ok(())
}

pub async fn filter_older_vms(state: &AppState) -> anyhow::Result<usize> {
    let servers: Vec<(Uuid, Uuid, String, String)> = sqlx::query_as(
        "SELECT s.id, s.integration_id, s.external_id, s.details->>'ipv4'
         FROM services s JOIN integrations i ON i.id = s.integration_id
         WHERE s.kind = 'vps' AND i.kind = 'proxmox' AND s.status IN ('active', 'suspended')
           AND s.external_id IS NOT NULL AND s.details ? 'ipv4' AND NOT s.details ? 'ip_filter'",
    )
    .fetch_all(&state.db)
    .await?;
    let mut done = 0;
    for (service_id, integration_id, vmid, ipv4) in servers {
        let result = async {
            let integrations::Driver::Proxmox(client) = integrations::driver(state, integration_id)
                .await
                .map_err(|error| anyhow::anyhow!("{error}"))?
            else {
                return Ok(());
            };
            client.bind_address(vmid.parse()?, ipv4.parse()?).await?;
            sqlx::query(
                "UPDATE services SET details = details || '{\"ip_filter\": true}'::jsonb WHERE id = $1",
            )
            .bind(service_id)
            .execute(&state.db)
            .await?;
            anyhow::Ok(())
        }
        .await;
        match result {
            Ok(()) => done += 1,
            Err(error) => {
                tracing::warn!(%service_id, error = %format!("{error:#}"), "turning on the ip filter failed");
            }
        }
    }
    Ok(done)
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
        "SELECT s.name, s.kind, s.integration_id, COALESCE(p.provisioning, '{}'::jsonb) AS provisioning, s.details,
                u.email::text AS email, u.email_verified_at IS NOT NULL AS email_verified,
                u.first_name, u.last_name, u.id AS user_id
         FROM services s
         JOIN users u ON u.id = s.user_id
         LEFT JOIN products p ON p.id = s.product_id
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
                email_verified: context.email_verified,
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
        "domain" => match &driver {
            integrations::Driver::Openprovider(client) => {
                provision_domain(state, integration_id, client, &context).await?
            }
            _ => anyhow::bail!("deze koppeling kan geen domeinen registreren"),
        },
        "vps" => match &driver {
            integrations::Driver::Proxmox(client) => {
                provision_vps(state, service_id, integration_id, client, &context).await?
            }
            integrations::Driver::OpenStack(client) => {
                provision_openstack(state, service_id, client, &context).await?
            }
            integrations::Driver::VirtFusion(client) => {
                provision_virtfusion(state, service_id, client, &context).await?
            }
            _ => anyhow::bail!("deze koppeling kan geen VPS maken"),
        },
        "volume" => provision_volume(state, &context, driver).await?,
        "floating_ip" => provision_floating_ip(state, service_id, integration_id, &driver).await?,
        "webhosting" => {
            let taken: bool = sqlx::query_scalar(
                "SELECT EXISTS (SELECT 1 FROM services WHERE kind = 'webhosting' AND id <> $1
                                AND integration_id = $2 AND lower(name) = lower($3)
                                AND status IN ('provisioning', 'active', 'suspended'))",
            )
            .bind(service_id)
            .bind(integration_id)
            .bind(&context.name)
            .fetch_one(&state.db)
            .await?;
            if taken {
                anyhow::bail!("deze domeinnaam draait al op een ander pakket op deze server");
            }
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
    if context.kind == "vps"
        && let Err(error) = crate::routes::vps_firewalls::apply_automatic(state, service_id).await
    {
        tracing::warn!(%service_id, error = %format!("{error:#}"), "applying automatic firewalls failed");
    }
    Ok(())
}

#[derive(Deserialize)]
struct VolumeOrder {
    size_gb: u32,
    server_id: Uuid,
}

async fn provision_volume(
    state: &AppState,
    context: &ProvisionContext,
    driver: integrations::Driver,
) -> anyhow::Result<pterodactyl::Provisioned> {
    let order: VolumeOrder = serde_json::from_value(context.details.clone())?;
    let storage = integrations::volumes::Storage::from_driver(driver)
        .ok_or_else(|| anyhow::anyhow!("deze koppeling kan geen volumes maken"))?;
    let server: Option<String> = sqlx::query_scalar(
        "SELECT external_id FROM services WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status = 'active'",
    )
    .bind(order.server_id)
    .bind(context.user_id)
    .fetch_optional(&state.db)
    .await?
    .flatten();
    let server =
        server.ok_or_else(|| anyhow::anyhow!("de server voor dit volume is niet actief"))?;
    let volume = storage
        .create(&server, &context.name, order.size_gb)
        .await?;
    Ok(pterodactyl::Provisioned {
        external_id: volume.id.clone(),
        details: json!({
            "size_gb": order.size_gb,
            "server_id": order.server_id,
            "host": volume.host,
        }),
    })
}

async fn provision_floating_ip(
    state: &AppState,
    service_id: Uuid,
    integration_id: Uuid,
    driver: &integrations::Driver,
) -> anyhow::Result<pterodactyl::Provisioned> {
    let (external_id, address) = match driver {
        integrations::Driver::Proxmox(client) => {
            let mut transaction = state.db.begin().await?;
            crate::locks::transaction(&mut transaction, format!("integration {integration_id}"))
                .await?;
            let current: Option<String> =
                sqlx::query_scalar("SELECT details->>'ipv4' FROM services WHERE id = $1")
                    .bind(service_id)
                    .fetch_one(&mut *transaction)
                    .await?;
            let address = match current {
                Some(address) => address,
                None => {
                    let taken: Vec<String> = sqlx::query_scalar(
                        "SELECT details->>'ipv4' FROM services
                         WHERE integration_id = $1 AND id <> $2 AND status <> 'terminated' AND details ? 'ipv4'",
                    )
                    .bind(integration_id)
                    .bind(service_id)
                    .fetch_all(&mut *transaction)
                    .await?;
                    let address = proxmox::ip_range(client.settings())
                        .map(|ip| ip.to_string())
                        .find(|ip| !taken.contains(ip))
                        .ok_or_else(|| {
                            anyhow::anyhow!("alle IP-adressen van deze koppeling zijn in gebruik")
                        })?;
                    sqlx::query(
                        "UPDATE services SET details = details || jsonb_build_object('ipv4', $2::text) WHERE id = $1",
                    )
                    .bind(service_id)
                    .bind(&address)
                    .execute(&mut *transaction)
                    .await?;
                    address
                }
            };
            transaction.commit().await?;
            (address.clone(), address)
        }
        integrations::Driver::OpenStack(client) => {
            let existing: Option<String> =
                sqlx::query_scalar("SELECT external_id FROM services WHERE id = $1")
                    .bind(service_id)
                    .fetch_one(&state.db)
                    .await?;
            let found = match existing {
                Some(id) => client.floating_ip(&id).await?,
                None => None,
            };
            match found {
                Some(found) => found,
                None => {
                    let created = client
                        .create_floating_ip(&format!("Stacklane {service_id}"))
                        .await?;
                    sqlx::query("UPDATE services SET external_id = $2 WHERE id = $1")
                        .bind(service_id)
                        .bind(&created.0)
                        .execute(&state.db)
                        .await?;
                    created
                }
            }
        }
        _ => anyhow::bail!("deze koppeling kan geen floating IPs maken"),
    };
    sqlx::query("UPDATE services SET name = $2 WHERE id = $1")
        .bind(service_id)
        .bind(&address)
        .execute(&state.db)
        .await?;
    Ok(pterodactyl::Provisioned {
        external_id,
        details: json!({ "ipv4": address }),
    })
}

#[derive(Deserialize, Default)]
struct VpsOrder {
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    ssh_key_ids: Vec<Uuid>,
    #[serde(default)]
    vmid: Option<u32>,
    #[serde(default)]
    server_id: Option<String>,
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

pub fn root_password() -> String {
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
        .as_deref()
        .and_then(|wanted| {
            settings
                .images
                .iter()
                .find(|image| image.template.to_string() == wanted)
        })
        .or_else(|| settings.images.first())
        .ok_or_else(|| anyhow::anyhow!("er staat geen besturingssysteem op deze koppeling"))?
        .clone();

    let mut transaction = state.db.begin().await?;
    crate::locks::transaction(&mut transaction, format!("integration {integration_id}")).await?;
    let recorded: Vec<String> = sqlx::query_scalar(
        "SELECT ids.id FROM services, LATERAL (VALUES (external_id), (details->>'vmid'), (details->>'rebuild_scratch')) AS ids(id)
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

    let ssh_keys = customer_keys(state, context, &order).await?;
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
        "ip_filter": true,
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

pub const REBUILD_FAILED: &str =
    "Herbouwen is mislukt. Probeer het opnieuw of neem contact op met support.";
pub const REBUILD_INTERRUPTED: &str =
    "Stacklane werd herstart tijdens het herbouwen. Probeer het opnieuw.";

pub struct RebuildRequest {
    pub image: String,
    pub ssh_key_ids: Vec<Uuid>,
}

pub async fn start_rebuild(
    state: &AppState,
    service_id: Uuid,
    owner: Uuid,
    request: RebuildRequest,
) -> AppResult<()> {
    let row: Option<(Uuid, String, Value)> = sqlx::query_as(
        "SELECT integration_id, external_id, details FROM services
         WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status = 'active'
           AND integration_id IS NOT NULL AND external_id IS NOT NULL",
    )
    .bind(service_id)
    .bind(owner)
    .fetch_optional(&state.db)
    .await?;
    let (integration_id, external_id, details) =
        row.ok_or_else(|| AppError::field("server", "Deze server is nu niet beschikbaar."))?;
    let driver = integrations::driver(state, integration_id).await?;
    let image_name = match &driver {
        integrations::Driver::Proxmox(client) => client
            .settings()
            .images
            .iter()
            .find(|image| image.template.to_string() == request.image)
            .map(|image| image.name.clone()),
        integrations::Driver::OpenStack(client) => client
            .settings()
            .images
            .iter()
            .find(|image| image.id == request.image)
            .map(|image| image.name.clone()),
        _ => {
            return Err(AppError::field(
                "image",
                "Deze server kun je alleen in het panel van VirtFusion herbouwen.",
            ));
        }
    }
    .ok_or_else(|| AppError::field("image", "Kies een besturingssysteem."))?;
    let keys: Vec<String> =
        sqlx::query_scalar("SELECT public_key FROM ssh_keys WHERE user_id = $1 AND id = ANY($2)")
            .bind(owner)
            .bind(&request.ssh_key_ids)
            .fetch_all(&state.db)
            .await?;
    if keys.len() != request.ssh_key_ids.len() {
        return Err(AppError::field(
            "ssh_key_ids",
            "Kies je eigen SSH-sleutels.",
        ));
    }
    let guard = crate::locks::service(&state.db, service_id).await?;
    let claimed = sqlx::query(
        "UPDATE services SET details = details || '{\"rebuilding\": true}'::jsonb, last_error = NULL, updated_at = now()
         WHERE id = $1 AND status = 'active' AND NOT details ? 'rebuilding'",
    )
    .bind(service_id)
    .execute(&state.db)
    .await?
    .rows_affected();
    guard.commit().await?;
    if claimed == 0 {
        return Err(AppError::field("server", "Deze server wordt al herbouwd."));
    }
    let state = state.clone();
    tokio::spawn(async move {
        let result = rebuild(
            &state,
            service_id,
            integration_id,
            &driver,
            &external_id,
            &details,
            &request.image,
            &keys,
        )
        .await;
        let outcome = match result {
            Ok(password) => {
                let concealed = crate::payments::conceal(&state, &password).ok();
                sqlx::query(
                    "UPDATE services SET details = (details - 'rebuilding' - 'rebuild_scratch')
                            || jsonb_build_object('image', $2::text)
                            || CASE WHEN $3::text IS NULL THEN '{}'::jsonb
                                    ELSE jsonb_build_object('root_password', $3::text) END,
                            updated_at = now()
                     WHERE id = $1",
                )
                .bind(service_id)
                .bind(&image_name)
                .bind(concealed)
                .execute(&state.db)
                .await
            }
            Err(error) => {
                tracing::warn!(%service_id, error = %format!("{error:#}"), "rebuilding a server failed");
                sqlx::query(
                    "UPDATE services SET details = details - 'rebuilding' - 'rebuild_scratch',
                            last_error = $2, updated_at = now()
                     WHERE id = $1",
                )
                .bind(service_id)
                .bind(REBUILD_FAILED)
                .execute(&state.db)
                .await
            }
        };
        if let Err(error) = outcome {
            tracing::error!(%service_id, ?error, "recording a rebuild failed");
        }
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn rebuild(
    state: &AppState,
    service_id: Uuid,
    integration_id: Uuid,
    driver: &integrations::Driver,
    external_id: &str,
    details: &Value,
    image: &str,
    keys: &[String],
) -> anyhow::Result<String> {
    let password = root_password();
    match driver {
        integrations::Driver::Proxmox(client) => {
            let vmid: u32 = external_id.parse()?;
            let template: u32 = image.parse()?;
            let mut transaction = state.db.begin().await?;
            crate::locks::transaction(&mut transaction, format!("integration {integration_id}"))
                .await?;
            let recorded: Vec<String> = sqlx::query_scalar(
                "SELECT ids.id FROM services, LATERAL (VALUES (external_id), (details->>'vmid'), (details->>'rebuild_scratch')) AS ids(id)
                 WHERE integration_id = $1 AND kind = 'vps' AND status <> 'terminated' AND ids.id IS NOT NULL",
            )
            .bind(integration_id)
            .fetch_all(&mut *transaction)
            .await?;
            let recorded: Vec<u32> = recorded.iter().filter_map(|id| id.parse().ok()).collect();
            let scratch = client.free_vmid(&recorded).await?;
            sqlx::query(
                "UPDATE services SET details = details || jsonb_build_object('rebuild_scratch', $2::text) WHERE id = $1",
            )
            .bind(service_id)
            .bind(scratch.to_string())
            .execute(&mut *transaction)
            .await?;
            transaction.commit().await?;
            client
                .rebuild(&proxmox::Rebuild {
                    vmid,
                    scratch,
                    template,
                    password: &password,
                    ssh_keys: keys,
                    disk_gb: details["disk_gb"]
                        .as_u64()
                        .and_then(|size| u32::try_from(size).ok())
                        .unwrap_or(10),
                })
                .await?;
        }
        integrations::Driver::OpenStack(client) => {
            let hostname = details["hostname"].as_str().unwrap_or("server");
            client
                .rebuild(external_id, image, hostname, &password, keys)
                .await?;
        }
        _ => anyhow::bail!("deze koppeling kan geen servers herbouwen"),
    }
    Ok(password)
}

async fn customer_keys(
    state: &AppState,
    context: &ProvisionContext,
    order: &VpsOrder,
) -> anyhow::Result<Vec<String>> {
    Ok(
        sqlx::query_scalar("SELECT public_key FROM ssh_keys WHERE user_id = $1 AND id = ANY($2)")
            .bind(context.user_id)
            .bind(&order.ssh_key_ids)
            .fetch_all(&state.db)
            .await?,
    )
}

async fn provision_openstack(
    state: &AppState,
    service_id: Uuid,
    client: &openstack::OpenStack,
    context: &ProvisionContext,
) -> anyhow::Result<pterodactyl::Provisioned> {
    let config: openstack::ProductConfig = serde_json::from_value(context.provisioning.clone())?;
    let order: VpsOrder = serde_json::from_value(context.details.clone()).unwrap_or_default();
    let settings = client.settings();
    let image = order
        .image
        .as_deref()
        .and_then(|wanted| settings.images.iter().find(|image| image.id == wanted))
        .or_else(|| settings.images.first())
        .ok_or_else(|| anyhow::anyhow!("er staat geen besturingssysteem op deze koppeling"))?
        .clone();

    if let Some(previous) = order.server_id.as_deref() {
        let _ = client.destroy(previous).await;
    }

    let ssh_keys = customer_keys(state, context, &order).await?;
    let password = root_password();
    let hostname = hostname(&context.name);
    let server_id = client
        .create(&openstack::NewServer {
            hostname: &hostname,
            image_id: &image.id,
            flavor_id: &config.flavor_id,
            ssh_keys: &ssh_keys,
            password: &password,
            description: &format!("Stacklane {service_id} ({})", context.email),
        })
        .await?;
    sqlx::query("UPDATE services SET details = details || $2 WHERE id = $1")
        .bind(service_id)
        .bind(json!({ "server_id": server_id }))
        .execute(&state.db)
        .await?;
    let ipv4 = client.wait_until_active(&server_id).await?;

    let mut details = context.details.clone();
    let extra = json!({
        "server_id": server_id,
        "ipv4": ipv4,
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
        external_id: server_id,
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

async fn provision_virtfusion(
    state: &AppState,
    service_id: Uuid,
    client: &virtfusion::VirtFusion,
    context: &ProvisionContext,
) -> anyhow::Result<pterodactyl::Provisioned> {
    let config: virtfusion::ProductConfig = serde_json::from_value(context.provisioning.clone())?;
    let order: VpsOrder = serde_json::from_value(context.details.clone()).unwrap_or_default();
    let image = order
        .image
        .as_deref()
        .and_then(|wanted| {
            client
                .settings()
                .images
                .iter()
                .find(|image| image.template.to_string() == wanted)
        })
        .or_else(|| client.settings().images.first())
        .ok_or_else(|| anyhow::anyhow!("er staat geen besturingssysteem op deze koppeling"))?
        .clone();

    if let Some(previous) = order.server_id.as_deref() {
        let _ = client.destroy(previous).await;
    }

    let keys: Vec<(String, String)> =
        sqlx::query_as("SELECT name, public_key FROM ssh_keys WHERE user_id = $1 AND id = ANY($2)")
            .bind(context.user_id)
            .bind(&order.ssh_key_ids)
            .fetch_all(&state.db)
            .await?;
    let reference = context.user_id.to_string();
    let name = format!("{} {}", context.first_name, context.last_name);
    let created = client
        .create(
            &virtfusion::Owner {
                reference: &reference,
                name: name.trim(),
                email: &context.email,
            },
            &config,
            &keys,
        )
        .await?;
    let server_id = created.id.to_string();
    sqlx::query("UPDATE services SET details = details || $2 WHERE id = $1")
        .bind(service_id)
        .bind(json!({ "server_id": server_id }))
        .execute(&state.db)
        .await?;
    let hostname = hostname(&context.name);
    let ipv4 = match client
        .build(created.id, image.template, &hostname, &created.key_ids)
        .await?
    {
        Some(ipv4) => Some(ipv4),
        None => client.ipv4(created.id).await?,
    };

    let mut details = context.details.clone();
    let extra = json!({
        "server_id": server_id,
        "ipv4": ipv4,
        "hostname": hostname,
        "image": image.name,
        "cores": config.cores,
        "memory_mb": config.memory_mb,
        "disk_gb": config.disk_gb,
    });
    if let (Some(target), Some(source)) = (details.as_object_mut(), extra.as_object()) {
        target.extend(source.clone());
    }
    Ok(pterodactyl::Provisioned {
        external_id: server_id,
        details,
    })
}

pub async fn registrar_handle(
    state: &AppState,
    integration_id: Uuid,
    client: &openprovider::Openprovider,
    profile: &crate::routes::whois_profiles::WhoisProfile,
) -> anyhow::Result<String> {
    let existing: Option<String> = sqlx::query_scalar(
        "SELECT handle FROM whois_profile_handles WHERE whois_profile_id = $1 AND integration_id = $2",
    )
    .bind(profile.id)
    .bind(integration_id)
    .fetch_optional(&state.db)
    .await?;
    if let Some(handle) = existing {
        return Ok(handle);
    }
    let handle = client.create_handle(&profile.contact()).await?;
    sqlx::query(
        "INSERT INTO whois_profile_handles (whois_profile_id, integration_id, handle) VALUES ($1, $2, $3)
         ON CONFLICT (whois_profile_id, integration_id) DO UPDATE SET handle = EXCLUDED.handle",
    )
    .bind(profile.id)
    .bind(integration_id)
    .bind(&handle)
    .execute(&state.db)
    .await?;
    Ok(handle)
}

#[derive(Deserialize)]
struct DomainOrder {
    whois_profile_id: Uuid,
    operation: String,
    #[serde(default)]
    auth_code: Option<String>,
}

async fn provision_domain(
    state: &AppState,
    integration_id: Uuid,
    client: &openprovider::Openprovider,
    context: &ProvisionContext,
) -> anyhow::Result<pterodactyl::Provisioned> {
    let order: DomainOrder = serde_json::from_value(context.details.clone())?;
    let profile =
        crate::routes::whois_profiles::load(state, context.user_id, order.whois_profile_id)
            .await
            .map_err(|_| anyhow::anyhow!("het WHOIS-profiel bestaat niet meer"))?;
    let handle = registrar_handle(state, integration_id, client, &profile).await?;
    let nameservers = client.settings().nameservers.clone();
    let registered = if order.operation == "transfer" {
        let code = crate::payments::reveal(state, order.auth_code.as_ref())
            .ok_or_else(|| anyhow::anyhow!("de autorisatiecode ontbreekt"))?;
        client
            .transfer(&context.name, &code, &handle, &nameservers)
            .await?
    } else {
        client
            .register(&context.name, &handle, &nameservers)
            .await?
    };

    let mut details = context.details.clone();
    if let Some(object) = details.as_object_mut() {
        object.remove("auth_code");
        object.insert("registrar_status".to_owned(), json!(registered.status));
        object.insert("expires_on".to_owned(), json!(registered.expires_on));
        object.insert("nameservers".to_owned(), json!(nameservers));
    }
    Ok(pterodactyl::Provisioned {
        external_id: registered.id.to_string(),
        details,
    })
}

async fn renew_domain(state: &AppState, service_id: Uuid) -> anyhow::Result<()> {
    let (integration_id, external_id): (Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT integration_id, external_id FROM services WHERE id = $1 AND kind = 'domain'",
    )
    .bind(service_id)
    .fetch_one(&state.db)
    .await?;
    let (Some(integration_id), Some(external_id)) = (integration_id, external_id) else {
        anyhow::bail!("dit domein is nooit geregistreerd");
    };
    let integrations::Driver::Openprovider(client) = integrations::driver(state, integration_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?
    else {
        anyhow::bail!("deze koppeling kan geen domeinen verlengen");
    };
    client.renew(&external_id).await?;
    let expires_on = client
        .domain(&external_id)
        .await
        .ok()
        .and_then(|domain| domain.expires_on);
    sqlx::query(
        "UPDATE services SET details = details || $2, last_error = NULL, updated_at = now() WHERE id = $1",
    )
    .bind(service_id)
    .bind(json!({ "expires_on": expires_on }))
    .execute(&state.db)
    .await?;
    Ok(())
}

pub fn renew_domain_in_background(state: AppState, service_id: Uuid) {
    tokio::spawn(async move {
        if let Err(error) = renew_domain(&state, service_id).await {
            tracing::warn!(%service_id, error = %format!("{error:#}"), "renewing a domain failed");
            let _ = sqlx::query("UPDATE services SET last_error = $2 WHERE id = $1")
                .bind(service_id)
                .bind(format!("Verlengen mislukt: {error:#}"))
                .execute(&state.db)
                .await;
            let admins: Vec<Uuid> = sqlx::query_scalar(
                "SELECT id FROM users WHERE role = 'admin' AND deleted_at IS NULL",
            )
            .fetch_all(&state.db)
            .await
            .unwrap_or_default();
            for admin in admins {
                crate::renewals::notify(
                    &state,
                    admin,
                    (
                        "Domein niet verlengd".to_owned(),
                        "Een klant heeft een domein betaald, maar de registrar wilde het niet verlengen. Verleng het met de hand voordat het verloopt.".to_owned(),
                    ),
                    (
                        "Domain not renewed".to_owned(),
                        "A customer paid for a domain but the registrar refused the renewal. Renew it by hand before it expires.".to_owned(),
                    ),
                    "/admin/services",
                )
                .await
                .ok();
            }
        }
    });
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

async fn storage_for(
    state: &AppState,
    integration_id: Uuid,
) -> anyhow::Result<integrations::volumes::Storage> {
    let driver = integrations::driver(state, integration_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    integrations::volumes::Storage::from_driver(driver)
        .ok_or_else(|| anyhow::anyhow!("deze koppeling kan geen volumes beheren"))
}

async fn server_reference(state: &AppState, server_id: &str) -> anyhow::Result<Option<String>> {
    Ok(sqlx::query_scalar(
        "SELECT external_id FROM services WHERE id = $1::uuid AND kind = 'vps' AND status <> 'terminated'",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .flatten())
}

async fn suspend_volume(
    state: &AppState,
    service_id: Uuid,
    integration_id: Uuid,
    external_id: &str,
    details: &Value,
) -> anyhow::Result<()> {
    let Some(server_id) = details["server_id"].as_str() else {
        return Ok(());
    };
    if let Some(server) = server_reference(state, server_id).await? {
        let volume = integrations::volumes::VolumeRef {
            host: details["host"].as_str().unwrap_or_default().to_owned(),
            id: external_id.to_owned(),
        };
        storage_for(state, integration_id)
            .await?
            .detach(&volume, &server)
            .await?;
    }
    sqlx::query(
        "UPDATE services SET details = (details - 'server_id') || jsonb_build_object('suspended_from', $2::text)
         WHERE id = $1",
    )
    .bind(service_id)
    .bind(server_id)
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn unsuspend_volume(
    state: &AppState,
    service_id: Uuid,
    integration_id: Uuid,
    external_id: &str,
    details: &Value,
) -> anyhow::Result<()> {
    let Some(server_id) = details["suspended_from"].as_str() else {
        return Ok(());
    };
    let target: Option<(Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT integration_id, external_id FROM services WHERE id = $1::uuid AND kind = 'vps' AND status = 'active'",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    if let Some((Some(target_integration), Some(server))) = target
        && target_integration == integration_id
    {
        let volume = integrations::volumes::VolumeRef {
            host: details["host"].as_str().unwrap_or_default().to_owned(),
            id: external_id.to_owned(),
        };
        let moved = storage_for(state, integration_id)
            .await?
            .attach(&volume, &server)
            .await?;
        sqlx::query(
            "UPDATE services SET external_id = $2,
                    details = (details - 'suspended_from') || jsonb_build_object('server_id', $3::text, 'host', $4::text)
             WHERE id = $1",
        )
        .bind(service_id)
        .bind(&moved.id)
        .bind(server_id)
        .bind(&moved.host)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query("UPDATE services SET details = details - 'suspended_from' WHERE id = $1")
            .bind(service_id)
            .execute(&state.db)
            .await?;
    }
    Ok(())
}

async fn delete_volume(
    state: &AppState,
    integration_id: Uuid,
    external_id: &str,
    details: &Value,
) -> anyhow::Result<()> {
    let driver = integrations::driver(state, integration_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let storage = integrations::volumes::Storage::from_driver(driver)
        .ok_or_else(|| anyhow::anyhow!("deze koppeling kan geen volumes beheren"))?;
    let server: Option<String> = match details["server_id"].as_str() {
        Some(server_id) => {
            sqlx::query_scalar("SELECT external_id FROM services WHERE id = $1::uuid")
                .bind(server_id)
                .fetch_optional(&state.db)
                .await?
                .flatten()
        }
        None => None,
    };
    let volume = integrations::volumes::VolumeRef {
        host: details["host"].as_str().unwrap_or_default().to_owned(),
        id: external_id.to_owned(),
    };
    storage.delete(&volume, server.as_deref()).await
}

async fn addresses_for(
    state: &AppState,
    integration_id: Uuid,
) -> anyhow::Result<integrations::floating_ips::Addresses> {
    let driver = integrations::driver(state, integration_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    integrations::floating_ips::Addresses::from_driver(driver)
        .ok_or_else(|| anyhow::anyhow!("deze koppeling kan geen floating IPs beheren"))
}

fn floating_ip(external_id: &str, details: &Value) -> integrations::floating_ips::FloatingIp {
    integrations::floating_ips::FloatingIp {
        id: external_id.to_owned(),
        address: details["ipv4"].as_str().unwrap_or_default().to_owned(),
    }
}

async fn suspend_floating_ip(
    state: &AppState,
    service_id: Uuid,
    integration_id: Uuid,
    external_id: &str,
    details: &Value,
) -> anyhow::Result<()> {
    let Some(server_id) = details["server_id"].as_str() else {
        return Ok(());
    };
    if let Some(server) = server_reference(state, server_id).await? {
        addresses_for(state, integration_id)
            .await?
            .detach(&floating_ip(external_id, details), &server)
            .await?;
    }
    sqlx::query(
        "UPDATE services SET details = (details - 'server_id') || jsonb_build_object('suspended_from', $2::text)
         WHERE id = $1",
    )
    .bind(service_id)
    .bind(server_id)
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn unsuspend_floating_ip(
    state: &AppState,
    service_id: Uuid,
    integration_id: Uuid,
    external_id: &str,
    details: &Value,
) -> anyhow::Result<()> {
    let Some(server_id) = details["suspended_from"].as_str() else {
        return Ok(());
    };
    let target: Option<(Option<Uuid>, Option<String>)> = sqlx::query_as(
        "SELECT integration_id, external_id FROM services WHERE id = $1::uuid AND kind = 'vps' AND status = 'active'",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?;
    if let Some((Some(target_integration), Some(server))) = target
        && target_integration == integration_id
    {
        addresses_for(state, integration_id)
            .await?
            .attach(&floating_ip(external_id, details), &server)
            .await?;
        sqlx::query(
            "UPDATE services SET details = (details - 'suspended_from') || jsonb_build_object('server_id', $2::text)
             WHERE id = $1",
        )
        .bind(service_id)
        .bind(server_id)
        .execute(&state.db)
        .await?;
    } else {
        sqlx::query("UPDATE services SET details = details - 'suspended_from' WHERE id = $1")
            .bind(service_id)
            .execute(&state.db)
            .await?;
    }
    Ok(())
}

async fn release_floating_ip(
    state: &AppState,
    integration_id: Uuid,
    external_id: &str,
    details: &Value,
) -> anyhow::Result<()> {
    let server = match details["server_id"].as_str() {
        Some(server_id) => server_reference(state, server_id).await?,
        None => None,
    };
    addresses_for(state, integration_id)
        .await?
        .release(&floating_ip(external_id, details), server.as_deref())
        .await
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
    let guard = crate::locks::service(&state.db, service_id).await?;
    let (integration_id, external_id, status, kind, details): (
        Option<Uuid>,
        Option<String>,
        String,
        String,
        Value,
    ) = sqlx::query_as(
        "SELECT integration_id, external_id, status, kind, details FROM services WHERE id = $1",
    )
    .bind(service_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if details.get("rebuilding").is_some() {
        return Err(AppError::field(
            "action",
            "Deze server wordt nu herbouwd. Probeer het daarna opnieuw.",
        ));
    }

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

    if kind == "volume" {
        if let (Some(integration_id), Some(external_id)) = (integration_id, external_id.as_deref())
        {
            let result = match action {
                "suspend" => {
                    suspend_volume(state, service_id, integration_id, external_id, &details).await
                }
                "unsuspend" => {
                    unsuspend_volume(state, service_id, integration_id, external_id, &details).await
                }
                _ => delete_volume(state, integration_id, external_id, &details).await,
            };
            result.map_err(|error| AppError::field("action", format!("{error:#}")))?;
        }
    } else if kind == "floating_ip" {
        if let (Some(integration_id), Some(external_id)) = (integration_id, external_id.as_deref())
        {
            let result = match action {
                "suspend" => {
                    suspend_floating_ip(state, service_id, integration_id, external_id, &details)
                        .await
                }
                "unsuspend" => {
                    unsuspend_floating_ip(state, service_id, integration_id, external_id, &details)
                        .await
                }
                _ => release_floating_ip(state, integration_id, external_id, &details).await,
            };
            result.map_err(|error| AppError::field("action", format!("{error:#}")))?;
        }
    } else if let (Some(integration_id), Some(external_id)) =
        (integration_id, external_id.as_deref())
    {
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
        if matches!(kind.as_str(), "vps" | "floating_ip")
            && let Some(ip) = details["ipv4"].as_str()
        {
            crate::routes::vps_rdns::clear(state, ip).await;
        }
    }
    if kind == "vps" && action == "terminate" {
        release_volumes(state, service_id, integration_id, external_id.as_deref()).await?;
        sqlx::query("DELETE FROM vps_network_servers WHERE service_id = $1")
            .bind(service_id)
            .execute(&state.db)
            .await?;
    }
    guard.commit().await?;
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

async fn release_volumes(
    state: &AppState,
    service_id: Uuid,
    integration_id: Option<Uuid>,
    external_id: Option<&str>,
) -> AppResult<()> {
    let lost: Vec<Option<Uuid>> = sqlx::query_scalar(
        "UPDATE services SET status = 'terminated', last_error = 'De server van dit volume is opgeheven.', updated_at = now()
         WHERE kind = 'volume' AND status <> 'terminated' AND integration_id = $1
           AND details->>'host' <> '' AND details->>'host' = $2
         RETURNING subscription_id",
    )
    .bind(integration_id)
    .bind(external_id)
    .fetch_all(&state.db)
    .await?;
    sqlx::query(
        "UPDATE subscriptions SET status = 'cancelled', cancelled_at = COALESCE(cancelled_at, now())
         WHERE id = ANY($1) AND status IN ('active', 'suspended', 'pending')",
    )
    .bind(lost.into_iter().flatten().collect::<Vec<_>>())
    .execute(&state.db)
    .await?;
    sqlx::query(
        "UPDATE services SET details = details - 'server_id' - 'suspended_from', updated_at = now()
         WHERE kind IN ('volume', 'floating_ip') AND status <> 'terminated'
           AND (details->>'server_id' = $1::text OR details->>'suspended_from' = $1::text)",
    )
    .bind(service_id.to_string())
    .execute(&state.db)
    .await?;
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
