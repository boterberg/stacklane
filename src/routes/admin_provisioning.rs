use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentAdmin;
use crate::error::{AppError, AppResult};
use crate::integrations::{
    self, IntegrationSummary, KINDS, cpanel, directadmin, openprovider, openstack, plesk, proxmox,
    pterodactyl, virtfusion, webhosting,
};
use crate::services;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateIntegration {
    kind: String,
    name: String,
    base_url: String,
    credentials: Value,
    #[serde(default)]
    settings: Value,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ProductRow {
    id: Uuid,
    kind: String,
    slug: String,
    name: String,
    description: Option<String>,
    monthly_price_cents: i32,
    is_active: bool,
    integration_id: Option<Uuid>,
    integration_name: Option<String>,
    provisioning: Value,
    service_count: i64,
}

#[derive(Deserialize)]
pub struct CreateProduct {
    kind: String,
    name: String,
    description: Option<String>,
    monthly_price_cents: i32,
    integration_id: Option<Uuid>,
    provisioning: Value,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ServiceRow {
    id: Uuid,
    kind: String,
    name: String,
    status: String,
    external_id: Option<String>,
    details: Value,
    last_error: Option<String>,
    product_name: Option<String>,
    integration_name: Option<String>,
    customer_name: String,
    customer_email: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct CreateService {
    customer_email: String,
    product_id: Uuid,
    name: String,
}

#[derive(Deserialize)]
pub struct CustomerQuery {
    q: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct CustomerOption {
    id: Uuid,
    email: String,
    name: String,
}

fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for character in name.to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    slug.trim_end_matches('-').to_owned()
}

const SERVICE_SELECT: &str = "SELECT s.id, s.kind, s.name, s.status, s.external_id, s.details, s.last_error,
        p.name AS product_name, i.name AS integration_name,
        COALESCE(NULLIF(trim(u.first_name || ' ' || u.last_name), ''), u.email::text) AS customer_name,
        u.email::text AS customer_email, s.created_at
    FROM services s
    JOIN users u ON u.id = s.user_id
    LEFT JOIN products p ON p.id = s.product_id
    LEFT JOIN integrations i ON i.id = s.integration_id";

pub async fn integrations(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<Vec<IntegrationSummary>>> {
    let rows = sqlx::query_as(
        "SELECT id, kind, name, base_url, settings, last_checked_at, last_error, created_at FROM integrations ORDER BY name",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub async fn create_integration(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(input): Json<CreateIntegration>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let mut errors = BTreeMap::new();
    if !KINDS.contains(&input.kind.as_str()) {
        errors.insert("kind", "Kies een type.".to_owned());
    } else if !matches!(
        input.kind.as_str(),
        "pterodactyl"
            | "proxmox"
            | "openstack"
            | "virtfusion"
            | "directadmin"
            | "plesk"
            | "openprovider"
            | "cpanel"
    ) {
        errors.insert(
            "kind",
            "Deze koppeling komt in een volgende versie.".to_owned(),
        );
    }
    if input.name.trim().is_empty() {
        errors.insert("name", "Geef de koppeling een naam.".to_owned());
    }
    let base_url = input.base_url.trim().trim_end_matches('/').to_owned();
    if !base_url.starts_with("https://") || base_url.len() <= "https://".len() {
        errors.insert(
            "base_url",
            "Vul een adres in dat met https:// begint.".to_owned(),
        );
    }
    let credentials_ok = match input.kind.as_str() {
        "pterodactyl" => serde_json::from_value::<pterodactyl::Credentials>(
            input.credentials.clone(),
        )
        .is_ok_and(|credentials| {
            !credentials.application_key.trim().is_empty()
                && !credentials.client_key.trim().is_empty()
        }),
        "proxmox" => serde_json::from_value::<proxmox::Credentials>(input.credentials.clone())
            .is_ok_and(|credentials| {
                credentials.token_id.contains('!') && !credentials.token_secret.trim().is_empty()
            }),
        "openprovider" => {
            serde_json::from_value::<openprovider::Credentials>(input.credentials.clone())
                .is_ok_and(|credentials| {
                    !credentials.username.trim().is_empty() && !credentials.password.is_empty()
                })
        }
        "virtfusion" => {
            serde_json::from_value::<virtfusion::Credentials>(input.credentials.clone())
                .is_ok_and(|credentials| !credentials.api_token.trim().is_empty())
        }
        "openstack" => serde_json::from_value::<openstack::Credentials>(input.credentials.clone())
            .is_ok_and(|credentials| {
                !credentials.application_credential_id.trim().is_empty()
                    && !credentials.application_credential_secret.trim().is_empty()
            }),
        "directadmin" => serde_json::from_value::<directadmin::Credentials>(
            input.credentials.clone(),
        )
        .is_ok_and(|credentials| {
            !credentials.username.trim().is_empty() && !credentials.login_key.is_empty()
        }),
        "cpanel" => serde_json::from_value::<cpanel::Credentials>(input.credentials.clone())
            .is_ok_and(|credentials| {
                !credentials.username.trim().is_empty() && !credentials.api_token.trim().is_empty()
            }),
        "plesk" => serde_json::from_value::<plesk::Credentials>(input.credentials.clone())
            .is_ok_and(|credentials| !credentials.api_key.trim().is_empty()),
        _ => true,
    };
    if !credentials_ok {
        errors.insert(
            "credentials",
            match input.kind.as_str() {
                "proxmox" => "Vul het token-ID (gebruiker@realm!naam) en het geheim in.",
                "openstack" => "Vul het ID en het geheim van de application credential in.",
                "virtfusion" => "Vul een API-token van VirtFusion in.",
                "directadmin" => "Vul de gebruikersnaam en een inlogsleutel van DirectAdmin in.",
                "plesk" => "Vul een API-sleutel van Plesk in.",
                "cpanel" => "Vul de WHM-gebruiker en een API-token in.",
                "openprovider" => "Vul de gebruikersnaam en het wachtwoord van Openprovider in.",
                _ => "Vul de application- en de client-API-sleutel in.",
            }
            .to_owned(),
        );
    }
    let settings = if input.kind == "openprovider" {
        let mut settings = serde_json::from_value::<openprovider::Settings>(input.settings.clone())
            .unwrap_or(openprovider::Settings {
                nameservers: openprovider::default_nameservers(),
            });
        settings.nameservers = settings
            .nameservers
            .iter()
            .map(|server| server.trim().trim_end_matches('.').to_lowercase())
            .filter(|server| !server.is_empty())
            .collect();
        if settings.nameservers.len() < 2
            || settings
                .nameservers
                .iter()
                .any(|server| openprovider::split_domain(server).is_none())
        {
            errors.insert(
                "settings",
                "Vul minstens twee geldige nameservers in.".to_owned(),
            );
        }
        serde_json::to_value(settings).map_err(anyhow::Error::from)?
    } else if input.kind == "virtfusion" {
        match serde_json::from_value::<virtfusion::Settings>(input.settings.clone()) {
            Ok(settings) if settings.hypervisor_group_id > 0 && !settings.images.is_empty() => {
                serde_json::to_value(settings).map_err(anyhow::Error::from)?
            }
            _ => {
                errors.insert(
                    "settings",
                    "Vul de hypervisorgroep en minstens één besturingssysteem in.".to_owned(),
                );
                Value::Null
            }
        }
    } else if input.kind == "openstack" {
        match serde_json::from_value::<openstack::Settings>(input.settings.clone()) {
            Ok(settings)
                if !settings.network_id.trim().is_empty() && !settings.images.is_empty() =>
            {
                serde_json::to_value(settings).map_err(anyhow::Error::from)?
            }
            _ => {
                errors.insert(
                    "settings",
                    "Vul het netwerk en minstens één image in.".to_owned(),
                );
                Value::Null
            }
        }
    } else if input.kind == "proxmox" {
        match serde_json::from_value::<proxmox::Settings>(input.settings) {
            Ok(settings)
                if settings.ip_from <= settings.ip_to && !settings.node.trim().is_empty() =>
            {
                serde_json::to_value(settings).map_err(anyhow::Error::from)?
            }
            _ => {
                errors.insert(
                    "settings",
                    "Vul de node, de gateway en een geldige reeks IP-adressen in.".to_owned(),
                );
                Value::Null
            }
        }
    } else if matches!(input.kind.as_str(), "directadmin" | "plesk") {
        let ip = input.settings["ip"]
            .as_str()
            .map(str::trim)
            .unwrap_or_default();
        if ip.parse::<std::net::IpAddr>().is_err() {
            errors.insert(
                "settings",
                "Vul het IP-adres in waarop nieuwe websites komen.".to_owned(),
            );
        }
        json!({ "ip": ip })
    } else {
        json!({})
    };
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }
    let credentials = serde_json::to_vec(&input.credentials).map_err(anyhow::Error::from)?;
    let summary = integrations::build(&input.kind, &base_url, &credentials, settings.clone())?
        .test()
        .await
        .map_err(|error| AppError::field("base_url", format!("{error:#}")))?;

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO integrations (id, kind, name, base_url, credentials, settings, last_checked_at)
         VALUES ($1, $2, $3, $4, $5, $6, now())",
    )
    .bind(id)
    .bind(&input.kind)
    .bind(input.name.trim())
    .bind(&base_url)
    .bind(integrations::seal_credentials(&state, &input.credentials)?)
    .bind(settings)
    .execute(&state.db)
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "id": id, "message": summary })),
    ))
}

pub async fn test_integration(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let driver = integrations::driver(&state, id).await?;
    let result = driver.test().await;
    let error = result.as_ref().err().map(|error| format!("{error:#}"));
    sqlx::query("UPDATE integrations SET last_checked_at = now(), last_error = $2 WHERE id = $1")
        .bind(id)
        .bind(&error)
        .execute(&state.db)
        .await?;
    Ok(Json(
        json!({ "ok": error.is_none(), "message": result.unwrap_or_else(|_| error.unwrap_or_default()) }),
    ))
}

pub async fn delete_integration(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let in_use: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM services WHERE integration_id = $1 AND status <> 'terminated')",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if in_use {
        return Err(AppError::field(
            "id",
            "Er draaien nog diensten op deze koppeling.",
        ));
    }
    sqlx::query("DELETE FROM integrations WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn catalog(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let failed = |error: anyhow::Error| AppError::field("integration", format!("{error:#}"));
    let driver = integrations::driver(&state, id).await?;
    if let Some(hosting) = webhosting::Hosting::from_driver(&driver) {
        return Ok(Json(hosting.plans().await.map_err(failed)?));
    }
    match driver {
        integrations::Driver::OpenStack(client) => {
            Ok(Json(client.flavors().await.map_err(failed)?))
        }
        integrations::Driver::VirtFusion(client) => {
            Ok(Json(client.packages().await.map_err(failed)?))
        }
        driver => {
            let catalog = driver.pterodactyl()?.catalog().await.map_err(failed)?;
            Ok(Json(
                serde_json::to_value(catalog).map_err(anyhow::Error::from)?,
            ))
        }
    }
}

pub async fn products(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<Vec<ProductRow>>> {
    let rows = sqlx::query_as(
        "SELECT p.id, p.kind, p.slug, p.name, p.description, p.monthly_price_cents, p.is_active, p.integration_id,
                i.name AS integration_name, p.provisioning,
                (SELECT count(*) FROM services s WHERE s.product_id = p.id AND s.status <> 'terminated') AS service_count
         FROM products p LEFT JOIN integrations i ON i.id = p.integration_id
         ORDER BY p.kind, p.monthly_price_cents",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub async fn create_product(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(input): Json<CreateProduct>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let mut errors = BTreeMap::new();
    if input.name.trim().is_empty() {
        errors.insert("name", "Geef het product een naam.".to_owned());
    }
    if input.monthly_price_cents < 0 {
        errors.insert(
            "monthly_price_cents",
            "De prijs kan niet negatief zijn.".to_owned(),
        );
    }
    if !matches!(
        input.kind.as_str(),
        "gameserver" | "vps" | "webhosting" | "volume" | "floating_ip"
    ) {
        errors.insert(
            "kind",
            "Op dit moment kun je gameserver-, VPS-, volume-, floating IP- en webhostingproducten aanmaken."
                .to_owned(),
        );
    }
    if input.integration_id.is_none() {
        errors.insert(
            "integration_id",
            "Kies op welke koppeling de servers komen.".to_owned(),
        );
    }
    let integration_kind: Option<String> = match input.integration_id {
        Some(id) => {
            sqlx::query_scalar("SELECT kind FROM integrations WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.db)
                .await?
        }
        None => None,
    };
    let specs = match (input.kind.as_str(), integration_kind.as_deref()) {
        ("webhosting", kind) => {
            let plan = input.provisioning["plan"]
                .as_str()
                .map(str::trim)
                .unwrap_or_default();
            if !matches!(kind, Some("cpanel" | "directadmin" | "plesk")) {
                errors.insert(
                    "integration_id",
                    "Kies een koppeling met cPanel, DirectAdmin of Plesk.".to_owned(),
                );
                Value::Null
            } else if plan.is_empty() {
                errors.insert("provisioning", "Kies een hostingpakket.".to_owned());
                Value::Null
            } else {
                json!({ "plan": plan })
            }
        }
        ("vps", Some("virtfusion")) => {
            match serde_json::from_value::<virtfusion::ProductConfig>(input.provisioning.clone()) {
                Ok(config) if config.package_id > 0 => virtfusion::product_details(&config),
                _ => {
                    errors.insert("provisioning", "Kies een pakket van VirtFusion.".to_owned());
                    Value::Null
                }
            }
        }
        ("volume", kind) => {
            if !matches!(kind, Some("proxmox" | "openstack")) {
                errors.insert(
                    "integration_id",
                    "Kies een koppeling met Proxmox VE of OpenStack.".to_owned(),
                );
            }
            json!({ "per_gb": true })
        }
        ("floating_ip", kind) => {
            let usable = match (kind, input.integration_id) {
                (Some("proxmox"), _) => true,
                (Some("openstack"), Some(id)) => sqlx::query_scalar::<_, Option<String>>(
                    "SELECT settings->>'floating_network_id' FROM integrations WHERE id = $1",
                )
                .bind(id)
                .fetch_one(&state.db)
                .await?
                .is_some_and(|network| !network.trim().is_empty()),
                _ => false,
            };
            if !usable {
                errors.insert(
                    "integration_id",
                    "Kies een koppeling met Proxmox VE, of OpenStack met een extern netwerk voor floating IPs."
                        .to_owned(),
                );
            }
            json!({ "per_ip": true })
        }
        ("vps", Some("openstack")) => {
            match serde_json::from_value::<openstack::ProductConfig>(input.provisioning.clone()) {
                Ok(config) if !config.flavor_id.trim().is_empty() => {
                    openstack::product_details(&config)
                }
                _ => {
                    errors.insert("provisioning", "Kies een flavor.".to_owned());
                    Value::Null
                }
            }
        }
        ("vps", _) => {
            match serde_json::from_value::<proxmox::ProductConfig>(input.provisioning.clone()) {
                Ok(config)
                    if config.cores >= 1 && config.memory_mb >= 256 && config.disk_gb >= 5 =>
                {
                    proxmox::product_details(&config)
                }
                _ => {
                    errors.insert(
                        "provisioning",
                        "Geef minstens 1 core, 256 MB geheugen en 5 GB schijf op.".to_owned(),
                    );
                    Value::Null
                }
            }
        }
        _ => match serde_json::from_value::<pterodactyl::ProductConfig>(input.provisioning.clone())
        {
            Ok(config)
                if config.memory >= 128
                    && config.disk >= 256
                    && config.egg_id > 0
                    && config.location_id > 0 =>
            {
                json!({ "memory": config.memory, "disk": config.disk, "cpu": config.cpu })
            }
            _ => {
                errors.insert(
                    "provisioning",
                    "Kies een locatie en een egg, met minstens 128 MB geheugen en 256 MB schijf."
                        .to_owned(),
                );
                Value::Null
            }
        },
    };
    if !errors.is_empty() {
        return Err(AppError::Validation(errors));
    }

    let slug = slugify(&input.name);
    let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM products WHERE slug = $1)")
        .bind(&slug)
        .fetch_one(&state.db)
        .await?;
    if taken {
        return Err(AppError::field(
            "name",
            "Er bestaat al een product met deze naam.",
        ));
    }
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO products (id, kind, slug, name, description, specs, monthly_price_cents, integration_id, provisioning)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(id)
    .bind(&input.kind)
    .bind(&slug)
    .bind(input.name.trim())
    .bind(input.description.filter(|value| !value.trim().is_empty()))
    .bind(specs)
    .bind(input.monthly_price_cents)
    .bind(input.integration_id)
    .bind(&input.provisioning)
    .execute(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

pub async fn services_index(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<Vec<ServiceRow>>> {
    let rows = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SERVICE_SELECT} ORDER BY s.created_at DESC LIMIT 200"
    )))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

pub async fn create_service(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(input): Json<CreateService>,
) -> AppResult<(StatusCode, Json<Value>)> {
    let user_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM users WHERE email = $1 AND role = 'customer' AND deleted_at IS NULL",
    )
    .bind(input.customer_email.trim().to_lowercase())
    .fetch_optional(&state.db)
    .await?;
    let user_id = user_id.ok_or_else(|| {
        AppError::field("customer_email", "Er is geen klant met dit e-mailadres.")
    })?;
    let product: Option<(String, Option<Uuid>)> =
        sqlx::query_as("SELECT kind, integration_id FROM products WHERE id = $1 AND is_active")
            .bind(input.product_id)
            .fetch_optional(&state.db)
            .await?;
    let (kind, integration_id) =
        product.ok_or_else(|| AppError::field("product_id", "Kies een product."))?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Geef de dienst een naam van hoogstens 60 tekens.",
        ));
    }

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO services (id, user_id, product_id, integration_id, kind, name, status) VALUES ($1, $2, $3, $4, $5, $6, 'pending')",
    )
    .bind(id)
    .bind(user_id)
    .bind(input.product_id)
    .bind(integration_id)
    .bind(kind)
    .bind(name)
    .execute(&state.db)
    .await?;
    services::provision_in_background(state.clone(), id);
    Ok((StatusCode::CREATED, Json(json!({ "id": id }))))
}

pub async fn service_action(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Path((id, action)): Path<(Uuid, String)>,
) -> AppResult<StatusCode> {
    match action.as_str() {
        "retry" => {
            let status: String = sqlx::query_scalar("SELECT status FROM services WHERE id = $1")
                .bind(id)
                .fetch_optional(&state.db)
                .await?
                .ok_or(AppError::NotFound)?;
            if status != "failed" {
                return Err(AppError::field(
                    "action",
                    "Alleen een mislukte dienst kun je opnieuw aanmaken.",
                ));
            }
            services::provision_in_background(state.clone(), id);
        }
        "suspend" | "unsuspend" | "terminate" => {
            services::change_state(&state, id, &action).await?
        }
        _ => return Err(AppError::NotFound),
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn customers(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Query(query): Query<CustomerQuery>,
) -> AppResult<Json<Vec<CustomerOption>>> {
    let search = format!("%{}%", query.q.unwrap_or_default().trim());
    let rows = sqlx::query_as(
        "SELECT id, email::text AS email, COALESCE(NULLIF(trim(first_name || ' ' || last_name), ''), email::text) AS name
         FROM users WHERE role = 'customer' AND deleted_at IS NULL AND (email::text ILIKE $1 OR first_name || ' ' || last_name ILIKE $1)
         ORDER BY created_at DESC LIMIT 20",
    )
    .bind(search)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(rows))
}

#[cfg(test)]
mod tests {
    use super::slugify;

    #[test]
    fn slugs_are_lowercase_and_dashed() {
        assert_eq!(slugify("Minecraft 4 GB"), "minecraft-4-gb");
        assert_eq!(slugify("  Paper — Iron!! "), "paper-iron");
    }
}
