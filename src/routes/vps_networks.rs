use std::net::Ipv4Addr;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::vps::machine_of;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, networks};
use crate::routes::vps_access::VpsActor;
use crate::state::AppState;

pub const MAX_NETWORKS: i64 = 10;
const MAX_PER_SERVER: i64 = 3;

#[derive(Serialize, sqlx::FromRow)]
pub struct Network {
    id: Uuid,
    name: String,
    cidr: String,
    integration_id: Uuid,
    platform: String,
    restart_needed: bool,
    servers: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Platform {
    integration_id: Uuid,
    name: String,
}

#[derive(Serialize)]
pub struct Overview {
    networks: Vec<Network>,
    platforms: Vec<Platform>,
    limit: i64,
    suggested_name: String,
    suggested_cidr: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Member {
    server_id: Uuid,
    server_name: String,
    address: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Candidate {
    server_id: Uuid,
    server_name: String,
}

#[derive(Serialize)]
pub struct Detail {
    #[serde(flatten)]
    network: Network,
    members: Vec<Member>,
    candidates: Vec<Candidate>,
}

#[derive(Deserialize)]
pub struct NewNetwork {
    name: String,
    cidr: String,
    integration_id: Uuid,
}

#[derive(Deserialize)]
pub struct Rename {
    name: String,
}

#[derive(Deserialize)]
pub struct Attach {
    server_id: Uuid,
}

pub fn private_cidr(input: &str) -> Option<(Ipv4Addr, u8)> {
    let (base, prefix) = input.trim().split_once('/')?;
    let base: Ipv4Addr = base.trim().parse().ok()?;
    let prefix: u8 = prefix.trim().parse().ok()?;
    if !(16..=28).contains(&prefix) {
        return None;
    }
    let network = Ipv4Addr::from(u32::from(base) & (u32::MAX << (32 - prefix)));
    network.is_private().then_some((network, prefix))
}

fn overlaps((a, a_prefix): (Ipv4Addr, u8), (b, b_prefix): (Ipv4Addr, u8)) -> bool {
    let mask = u32::MAX << (32 - a_prefix.min(b_prefix));
    u32::from(a) & mask == u32::from(b) & mask
}

fn hosts((network, prefix): (Ipv4Addr, u8)) -> impl Iterator<Item = Ipv4Addr> {
    let first = u32::from(network) + 2;
    let last = u32::from(network) + (1u32 << (32 - prefix)) - 2;
    (first..=last).map(Ipv4Addr::from)
}

fn valid_name(name: &str) -> AppResult<&str> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > 60
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
    {
        return Err(AppError::field(
            "name",
            "Gebruik hoogstens 60 letters, cijfers, spaties, punten, streepjes of liggende streepjes.",
        ));
    }
    Ok(name)
}

fn platform_unavailable() -> AppError {
    AppError::field(
        "integration_id",
        "Op dit platform kun je geen netwerken maken.",
    )
}

async fn platform(state: &AppState, integration_id: Uuid) -> AppResult<networks::Networks> {
    networks::Networks::from_driver(integrations::driver(state, integration_id).await?)
        .ok_or_else(platform_unavailable)
}

const NETWORK_SELECT: &str = "SELECT n.id, n.name, n.cidr, n.integration_id, i.name AS platform,
            i.kind = 'proxmox' AS restart_needed,
            (SELECT count(*) FROM vps_network_servers m WHERE m.network_id = n.id) AS servers,
            n.created_at
     FROM vps_networks n JOIN integrations i ON i.id = n.integration_id";

async fn platforms(state: &AppState) -> AppResult<Vec<Platform>> {
    let candidates: Vec<(Uuid, String, String, serde_json::Value)> = sqlx::query_as(
        "SELECT DISTINCT i.id, i.name, i.kind, i.settings FROM integrations i
         JOIN products p ON p.integration_id = i.id AND p.kind = 'vps' AND p.is_active
         WHERE i.kind IN ('proxmox', 'openstack')
         ORDER BY i.name, i.id",
    )
    .fetch_all(&state.db)
    .await?;
    Ok(candidates
        .into_iter()
        .filter(|(_, _, kind, settings)| {
            kind == "openstack"
                || serde_json::from_value::<integrations::proxmox::Settings>(settings.clone())
                    .ok()
                    .and_then(|settings| settings.private_bridge)
                    .is_some_and(|bridge| !bridge.trim().is_empty())
        })
        .map(|(integration_id, name, ..)| Platform {
            integration_id,
            name,
        })
        .collect())
}

pub async fn index(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Overview>> {
    actor.require("networks.view")?;
    let networks: Vec<Network> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{NETWORK_SELECT} WHERE n.user_id = $1 ORDER BY n.created_at, n.id"
    )))
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    let platforms = if actor.can("networks.create") {
        platforms(&state).await?
    } else {
        Vec::new()
    };
    let names: Vec<String> = networks
        .iter()
        .map(|network| network.name.to_lowercase())
        .collect();
    let suggested_name = (1..)
        .map(|index| format!("network-{index}"))
        .find(|name| !names.contains(name))
        .unwrap_or_default();
    let taken: Vec<(Ipv4Addr, u8)> = networks
        .iter()
        .filter_map(|network| private_cidr(&network.cidr))
        .collect();
    let suggested_cidr = (1..=255u8)
        .map(|second| (Ipv4Addr::new(10, second, 0, 0), 24))
        .find(|candidate| !taken.iter().any(|used| overlaps(*used, *candidate)))
        .map(|(network, prefix)| format!("{network}/{prefix}"))
        .unwrap_or_default();
    Ok(Json(Overview {
        networks,
        platforms,
        limit: MAX_NETWORKS,
        suggested_name,
        suggested_cidr,
    }))
}

pub async fn create(
    State(state): State<AppState>,
    actor: VpsActor,
    Json(input): Json<NewNetwork>,
) -> AppResult<(StatusCode, Json<Network>)> {
    actor.require("networks.create")?;
    let owner = actor.owner;
    crate::locks::exclusive(format!("networks {owner}"), async move {
        let name = valid_name(&input.name)?.to_owned();
        let cidr = private_cidr(&input.cidr).ok_or_else(|| {
            AppError::field(
                "cidr",
                "Kies een privé IP-bereik, zoals 10.1.0.0/24, met een netmasker van /16 tot /28.",
            )
        })?;
        if !platforms(&state)
            .await?
            .iter()
            .any(|platform| platform.integration_id == input.integration_id)
        {
            return Err(platform_unavailable());
        }
        let has_server: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM services WHERE user_id = $1 AND kind = 'vps'
                            AND status = 'active' AND integration_id = $2)",
        )
        .bind(owner)
        .bind(input.integration_id)
        .fetch_one(&state.db)
        .await?;
        if !has_server {
            return Err(AppError::field(
                "integration_id",
                "Je hebt een actieve VPS op dit platform nodig voor een privénetwerk.",
            ));
        }
        let existing: Vec<(String, String)> =
            sqlx::query_as("SELECT name, cidr FROM vps_networks WHERE user_id = $1")
                .bind(owner)
                .fetch_all(&state.db)
                .await?;
        if i64::try_from(existing.len()).unwrap_or(i64::MAX) >= MAX_NETWORKS {
            return Err(AppError::field(
                "name",
                "Je hebt al 10 netwerken. Verwijder er eerst een.",
            ));
        }
        if existing
            .iter()
            .any(|(other, _)| other.eq_ignore_ascii_case(&name))
        {
            return Err(AppError::field(
                "name",
                "Je hebt al een netwerk met deze naam.",
            ));
        }
        if existing
            .iter()
            .filter_map(|(_, other)| private_cidr(other))
            .any(|other| overlaps(other, cidr))
        {
            return Err(AppError::field(
                "cidr",
                "Dit IP-bereik overlapt met een van je andere netwerken.",
            ));
        }
        let cidr = format!("{}/{}", cidr.0, cidr.1);
        let platform = platform(&state, input.integration_id).await?;
        let id = Uuid::new_v4();
        let external_id = platform
            .create(&format!("stacklane-{id}"), &cidr)
            .await
            .map_err(|error| AppError::upstream("name", error))?;
        let mut transaction = state.db.begin().await?;
        let vlan = match platform.vlans() {
            Some(range) => {
                crate::locks::transaction(
                    &mut transaction,
                    format!("vlans {}", input.integration_id),
                )
                .await?;
                let used: Vec<i32> = sqlx::query_scalar(
                    "SELECT vlan FROM vps_networks WHERE integration_id = $1 AND vlan IS NOT NULL",
                )
                .bind(input.integration_id)
                .fetch_all(&mut *transaction)
                .await?;
                Some(
                    range
                        .map(i32::from)
                        .find(|vlan| !used.contains(vlan))
                        .ok_or_else(|| {
                            AppError::field(
                                "integration_id",
                                "Op dit platform is geen ruimte meer voor nieuwe netwerken.",
                            )
                        })?,
                )
            }
            None => None,
        };
        let stored = sqlx::query(
            "INSERT INTO vps_networks (id, user_id, integration_id, name, cidr, vlan, external_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(owner)
        .bind(input.integration_id)
        .bind(&name)
        .bind(&cidr)
        .bind(vlan)
        .bind(&external_id)
        .execute(&mut *transaction)
        .await;
        let committed = match stored {
            Ok(_) => transaction.commit().await.map_err(AppError::from),
            Err(error) => Err(error.into()),
        };
        if let Err(error) = committed {
            let created = networks::NetworkRef {
                vlan: None,
                external_id,
            };
            platform.delete(&created).await.ok();
            return Err(error);
        }
        let network: Network = sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{NETWORK_SELECT} WHERE n.id = $1"
        )))
        .bind(id)
        .fetch_one(&state.db)
        .await?;
        Ok((StatusCode::CREATED, Json(network)))
    })
    .await
}

struct Owned {
    integration_id: Uuid,
    cidr: String,
    reference: networks::NetworkRef,
}

async fn owned(state: &AppState, actor: &VpsActor, id: Uuid) -> AppResult<Owned> {
    let (integration_id, cidr, vlan, external_id): (Uuid, String, Option<i32>, Option<String>) =
        sqlx::query_as(
            "SELECT integration_id, cidr, vlan, external_id FROM vps_networks
             WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(actor.owner)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(Owned {
        integration_id,
        cidr,
        reference: networks::NetworkRef {
            vlan: vlan.and_then(|vlan| u16::try_from(vlan).ok()),
            external_id,
        },
    })
}

pub async fn show(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Detail>> {
    actor.require("networks.view")?;
    let network: Network = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{NETWORK_SELECT} WHERE n.id = $1 AND n.user_id = $2"
    )))
    .bind(id)
    .bind(actor.owner)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let members: Vec<Member> = sqlx::query_as(
        "SELECT s.id AS server_id, s.name AS server_name, m.address
         FROM vps_network_servers m JOIN services s ON s.id = m.service_id
         WHERE m.network_id = $1 ORDER BY s.name, m.created_at, s.id",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    let mut candidates = Vec::new();
    if actor.can("networks.attach") {
        let servers: Vec<Candidate> = sqlx::query_as(
            "SELECT id AS server_id, name AS server_name FROM services s
             WHERE user_id = $1 AND kind = 'vps' AND status = 'active' AND integration_id = $2
               AND NOT EXISTS (SELECT 1 FROM vps_network_servers m
                               WHERE m.network_id = $3 AND m.service_id = s.id)
             ORDER BY name, id",
        )
        .bind(actor.owner)
        .bind(network.integration_id)
        .bind(id)
        .fetch_all(&state.db)
        .await?;
        for server in servers {
            if let Ok(machine) = machine_of(&state, actor.owner, server.server_id).await
                && machine.features().networks
            {
                candidates.push(server);
            }
        }
    }
    Ok(Json(Detail {
        network,
        members,
        candidates,
    }))
}

pub async fn update(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<Rename>,
) -> AppResult<StatusCode> {
    actor.require("networks.update")?;
    let owner = actor.owner;
    crate::locks::exclusive(format!("networks {owner}"), async {
        owned(&state, &actor, id).await?;
        let name = valid_name(&input.name)?;
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM vps_networks WHERE user_id = $1 AND id <> $2 AND lower(name) = lower($3))",
        )
        .bind(owner)
        .bind(id)
        .bind(name)
        .fetch_one(&state.db)
        .await?;
        if taken {
            return Err(AppError::field(
                "name",
                "Je hebt al een netwerk met deze naam.",
            ));
        }
        sqlx::query("UPDATE vps_networks SET name = $2 WHERE id = $1")
            .bind(id)
            .bind(name)
            .execute(&state.db)
            .await?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

pub async fn destroy(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    actor.require("networks.delete")?;
    crate::locks::exclusive(format!("network {id}"), async {
        let network = owned(&state, &actor, id).await?;
        let attached: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM vps_network_servers WHERE network_id = $1)",
        )
        .bind(id)
        .fetch_one(&state.db)
        .await?;
        if attached {
            return Err(AppError::field(
                "network",
                "Koppel eerst alle servers los van dit netwerk.",
            ));
        }
        platform(&state, network.integration_id)
            .await?
            .delete(&network.reference)
            .await
            .map_err(|error| AppError::upstream("network", error))?;
        sqlx::query("DELETE FROM vps_networks WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

pub async fn attach(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<Attach>,
) -> AppResult<StatusCode> {
    actor.browser_only()?;
    actor.require("networks.attach")?;
    crate::locks::exclusive(format!("network {id}"), async {
        let network = owned(&state, &actor, id).await?;
        let target: Option<(Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT integration_id, external_id FROM services
             WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status = 'active'",
        )
        .bind(input.server_id)
        .bind(actor.owner)
        .fetch_optional(&state.db)
        .await?;
        let Some((Some(_), Some(_))) = target else {
            return Err(AppError::field(
                "server_id",
                "Kies een van je actieve servers.",
            ));
        };
        let _guard = crate::locks::service(&state.db, input.server_id).await?;
        let target: Option<(Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT integration_id, external_id FROM services
             WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status = 'active'
               AND NOT details ? 'rebuilding'",
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
        if integration_id != network.integration_id {
            return Err(AppError::field(
                "server_id",
                "Deze server staat op een ander platform dan het netwerk.",
            ));
        }
        let joined: Vec<Uuid> =
            sqlx::query_scalar("SELECT network_id FROM vps_network_servers WHERE service_id = $1")
                .bind(input.server_id)
                .fetch_all(&state.db)
                .await?;
        if joined.contains(&id) {
            return Ok(StatusCode::NO_CONTENT);
        }
        if i64::try_from(joined.len()).unwrap_or(i64::MAX) >= MAX_PER_SERVER {
            return Err(AppError::field(
                "server_id",
                "Deze server zit al in drie netwerken.",
            ));
        }
        let range = private_cidr(&network.cidr).ok_or_else(|| {
            AppError::field("network", "Het IP-bereik van dit netwerk klopt niet.")
        })?;
        let platform = platform(&state, network.integration_id).await?;
        let address = if network.reference.vlan.is_some() {
            let used: Vec<String> =
                sqlx::query_scalar("SELECT address FROM vps_network_servers WHERE network_id = $1")
                    .bind(id)
                    .fetch_all(&state.db)
                    .await?;
            let free = hosts(range)
                .map(|host| format!("{host}/{}", range.1))
                .find(|address| !used.contains(address))
                .ok_or_else(|| {
                    AppError::field("network", "Dit netwerk heeft geen vrije adressen meer.")
                })?;
            Some(free)
        } else {
            None
        };
        let attached = platform
            .attach(&network.reference, &server, address.as_deref())
            .await
            .map_err(|error| AppError::upstream("server_id", error))?;
        let address = if attached.address.contains('/') {
            attached.address.clone()
        } else {
            format!("{}/{}", attached.address, range.1)
        };
        let stored = sqlx::query(
            "INSERT INTO vps_network_servers (network_id, service_id, address, port)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(id)
        .bind(input.server_id)
        .bind(&address)
        .bind(&attached.port)
        .execute(&state.db)
        .await;
        if let Err(error) = stored {
            platform.detach(&server, &attached.port).await.ok();
            return Err(error.into());
        }
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

pub async fn detach(
    State(state): State<AppState>,
    actor: VpsActor,
    Path((id, server_id)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    actor.require("networks.detach")?;
    crate::locks::exclusive(format!("network {id}"), async {
        let network = owned(&state, &actor, id).await?;
        let _guard = crate::locks::service(&state.db, server_id).await?;
        let (port, server, status): (String, Option<String>, String) = sqlx::query_as(
            "SELECT m.port, s.external_id, s.status FROM vps_network_servers m
             JOIN services s ON s.id = m.service_id
             WHERE m.network_id = $1 AND m.service_id = $2",
        )
        .bind(id)
        .bind(server_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
        if let Some(server) = server.filter(|_| status != "terminated") {
            platform(&state, network.integration_id)
                .await?
                .detach(&server, &port)
                .await
                .map_err(|error| AppError::upstream("server_id", error))?;
        }
        sqlx::query("DELETE FROM vps_network_servers WHERE network_id = $1 AND service_id = $2")
            .bind(id)
            .bind(server_id)
            .execute(&state.db)
            .await?;
        sqlx::query(
            "UPDATE services SET details = details || jsonb_build_object('networks_changed_at', extract(epoch FROM now())::bigint)
             WHERE id = $1",
        )
        .bind(server_id)
        .execute(&state.db)
        .await?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ServerNetwork {
    network_id: Uuid,
    name: String,
    cidr: String,
    address: Option<String>,
}

#[derive(Serialize)]
pub struct ServerNetworks {
    attached: Vec<ServerNetwork>,
    available: Vec<ServerNetwork>,
}

pub async fn server(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ServerNetworks>> {
    actor.require("networks.view")?;
    let integration: Option<Uuid> = sqlx::query_scalar(
        "SELECT integration_id FROM services WHERE id = $1 AND user_id = $2 AND kind = 'vps'",
    )
    .bind(id)
    .bind(actor.owner)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let attached: Vec<ServerNetwork> = sqlx::query_as(
        "SELECT n.id AS network_id, n.name, n.cidr, m.address
         FROM vps_network_servers m JOIN vps_networks n ON n.id = m.network_id
         WHERE m.service_id = $1 AND n.user_id = $2
         ORDER BY n.name, n.id",
    )
    .bind(id)
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    let available: Vec<ServerNetwork> = sqlx::query_as(
        "SELECT n.id AS network_id, n.name, n.cidr, NULL::text AS address
         FROM vps_networks n
         WHERE n.user_id = $1 AND n.integration_id = $2
           AND NOT EXISTS (SELECT 1 FROM vps_network_servers m
                           WHERE m.network_id = n.id AND m.service_id = $3)
         ORDER BY n.name, n.id",
    )
    .bind(actor.owner)
    .bind(integration)
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(ServerNetworks {
        attached,
        available,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_private_ranges_and_normalises_them() {
        assert_eq!(
            private_cidr("10.1.0.7/24"),
            Some((Ipv4Addr::new(10, 1, 0, 0), 24))
        );
        assert_eq!(
            private_cidr(" 192.168.5.0 / 28 "),
            Some((Ipv4Addr::new(192, 168, 5, 0), 28))
        );
        assert!(private_cidr("172.32.0.0/24").is_none());
        assert!(private_cidr("8.8.8.0/24").is_none());
        assert!(private_cidr("10.0.0.0/8").is_none());
        assert!(private_cidr("10.0.0.0/30").is_none());
        assert!(private_cidr("10.0.0.0").is_none());
    }

    #[test]
    fn finds_overlaps_and_usable_hosts() {
        let a = private_cidr("10.1.0.0/16").unwrap();
        let b = private_cidr("10.1.4.0/24").unwrap();
        let c = private_cidr("10.2.0.0/24").unwrap();
        assert!(overlaps(a, b));
        assert!(overlaps(b, a));
        assert!(!overlaps(a, c));
        let small: Vec<String> = hosts(private_cidr("10.9.9.0/28").unwrap())
            .map(|host| host.to_string())
            .collect();
        assert_eq!(small.first().map(String::as_str), Some("10.9.9.2"));
        assert_eq!(small.last().map(String::as_str), Some("10.9.9.14"));
        assert_eq!(small.len(), 13);
    }
}

#[cfg(test)]
mod flows {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use axum::http::{Method, StatusCode};
    use serde_json::{Value, json};
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::testing::{Client, NewService, fake_api, proxmox_integration, service};

    static CONFIGS: Mutex<BTreeMap<String, BTreeMap<String, String>>> = Mutex::new(BTreeMap::new());

    fn node(method: &str, path: &str, body: &str) -> (u16, Value) {
        let Some(vm) = path
            .strip_prefix("/api2/json/nodes/pve/qemu/")
            .and_then(|rest| rest.strip_suffix("/config"))
        else {
            return (200, json!({ "data": null }));
        };
        let mut configs = CONFIGS.lock().unwrap();
        let config = configs.entry(vm.to_owned()).or_insert_with(|| {
            BTreeMap::from([("net0".to_owned(), "virtio,bridge=vmbr0".to_owned())])
        });
        if method == "POST" {
            for (key, value) in url::form_urlencoded::parse(body.as_bytes()) {
                if key == "delete" {
                    for gone in value.split(',') {
                        config.remove(gone);
                    }
                } else {
                    config.insert(key.into_owned(), value.into_owned());
                }
            }
            return (200, json!({ "data": null }));
        }
        (200, json!({ "data": config }))
    }

    fn config(vm: &str) -> BTreeMap<String, String> {
        CONFIGS.lock().unwrap().get(vm).cloned().unwrap_or_default()
    }

    async fn setup(client: &Client) -> Uuid {
        let (base, _) = fake_api(node).await;
        let integration = proxmox_integration(&client.state, &base).await;
        sqlx::query(
            "UPDATE integrations SET settings = settings || '{\"private_bridge\": \"vmbr1\", \"vlan_from\": 100, \"vlan_to\": 101}'
             WHERE id = $1",
        )
        .bind(integration)
        .execute(&client.state.db)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO products (id, kind, slug, name, monthly_price_cents, integration_id, specs, provisioning)
             VALUES ($1, 'vps', $1::text, 'VPS', 500, $2, '{}', '{}')",
        )
        .bind(Uuid::new_v4())
        .bind(integration)
        .execute(&client.state.db)
        .await
        .unwrap();
        integration
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn networks_get_their_own_vlan_and_refuse_overlaps(db: PgPool) {
        let client = Client::new(db);
        let integration = setup(&client).await;
        let (user, cookie) = client.register("net-create@example.test").await;
        let create = |name: &str, cidr: &str| json!({ "name": name, "cidr": cidr, "integration_id": integration });

        let without_server = client
            .send(
                Method::POST,
                "/vps/networks",
                Some(&cookie),
                Some(create("network-1", "10.1.0.0/24")),
            )
            .await;
        assert_eq!(without_server.status, StatusCode::UNPROCESSABLE_ENTITY);
        crate::testing::service(
            &client.state,
            crate::testing::NewService {
                user,
                subscription: None,
                kind: "vps",
                status: "active",
                integration: Some(integration),
                external_id: Some("700"),
                details: json!({}),
            },
        )
        .await;

        let overview = client
            .send(Method::GET, "/vps/networks", Some(&cookie), None)
            .await;
        assert_eq!(overview.body["suggested_name"], "network-1");
        assert_eq!(overview.body["suggested_cidr"], "10.1.0.0/24");
        assert_eq!(
            overview.body["platforms"][0]["integration_id"],
            integration.to_string()
        );

        let first = client
            .send(
                Method::POST,
                "/vps/networks",
                Some(&cookie),
                Some(create("network-1", "10.1.0.9/24")),
            )
            .await;
        assert_eq!(first.status, StatusCode::CREATED, "{}", first.body);
        assert_eq!(first.body["cidr"], "10.1.0.0/24");

        for (name, cidr, field) in [
            ("overlap", "10.1.0.0/16", "cidr"),
            ("public", "8.8.8.0/24", "cidr"),
            ("NETWORK-1", "10.2.0.0/24", "name"),
        ] {
            let refused = client
                .send(
                    Method::POST,
                    "/vps/networks",
                    Some(&cookie),
                    Some(create(name, cidr)),
                )
                .await;
            assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY, "{name}");
            assert!(
                refused.body["errors"][field].is_string(),
                "{name}: {}",
                refused.body
            );
        }

        let second = client
            .send(
                Method::POST,
                "/vps/networks",
                Some(&cookie),
                Some(create("network-2", "10.2.0.0/24")),
            )
            .await;
        assert_eq!(second.status, StatusCode::CREATED, "{}", second.body);
        let full = client
            .send(
                Method::POST,
                "/vps/networks",
                Some(&cookie),
                Some(create("network-3", "10.3.0.0/24")),
            )
            .await;
        assert!(
            full.body["errors"]["integration_id"].is_string(),
            "{}",
            full.body
        );

        let vlans: Vec<i32> = sqlx::query_scalar(
            "SELECT vlan FROM vps_networks WHERE integration_id = $1 ORDER BY vlan",
        )
        .bind(integration)
        .fetch_all(&client.state.db)
        .await
        .unwrap();
        assert_eq!(vlans, [100, 101]);
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn servers_join_a_network_on_a_tagged_card(db: PgPool) {
        let client = Client::new(db);
        let integration = setup(&client).await;
        let (user, cookie) = client.register("net-join@example.test").await;
        let server = |vmid: &'static str| NewService {
            user,
            subscription: None,
            kind: "vps",
            status: "active",
            integration: Some(integration),
            external_id: Some(vmid),
            details: json!({ "vmid": vmid.parse::<u32>().unwrap() }),
        };
        let first = service(&client.state, server("650")).await;
        let second = service(&client.state, server("651")).await;
        let network = client
            .send(
                Method::POST,
                "/vps/networks",
                Some(&cookie),
                Some(
                    json!({ "name": "lan", "cidr": "10.8.0.0/24", "integration_id": integration }),
                ),
            )
            .await
            .body["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let path = format!("/vps/networks/{network}");

        for server in [first, second] {
            let reply = client
                .send(
                    Method::POST,
                    &format!("{path}/servers"),
                    Some(&cookie),
                    Some(json!({ "server_id": server })),
                )
                .await;
            assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
        }
        assert_eq!(config("650")["net1"], "virtio,bridge=vmbr1,tag=100");
        assert_eq!(config("650")["ipconfig1"], "ip=10.8.0.2/24");
        assert_eq!(config("651")["ipconfig1"], "ip=10.8.0.3/24");

        let detail = client.send(Method::GET, &path, Some(&cookie), None).await;
        let addresses: Vec<&str> = detail.body["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|member| member["address"].as_str())
            .collect();
        assert_eq!(addresses, ["10.8.0.2/24", "10.8.0.3/24"]);
        assert_eq!(detail.body["restart_needed"], true);

        let own = client
            .send(
                Method::GET,
                &format!("/vps/servers/{first}/networks"),
                Some(&cookie),
                None,
            )
            .await;
        assert_eq!(own.body["attached"][0]["address"], "10.8.0.2/24");
        assert_eq!(own.body["available"].as_array().map(Vec::len), Some(0));

        let (_, stranger) = client.register("net-stranger@example.test").await;
        let foreign = client
            .send(
                Method::GET,
                &format!("/vps/servers/{first}/networks"),
                Some(&stranger),
                None,
            )
            .await;
        assert_eq!(foreign.status, StatusCode::NOT_FOUND);
        let peek = client.send(Method::GET, &path, Some(&stranger), None).await;
        assert_eq!(peek.status, StatusCode::NOT_FOUND);

        let busy = client
            .send(Method::DELETE, &path, Some(&cookie), None)
            .await;
        assert_eq!(busy.status, StatusCode::UNPROCESSABLE_ENTITY);

        for server in [first, second] {
            let reply = client
                .send(
                    Method::DELETE,
                    &format!("{path}/servers/{server}"),
                    Some(&cookie),
                    None,
                )
                .await;
            assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
        }
        assert!(!config("650").contains_key("net1"));
        assert!(!config("650").contains_key("ipconfig1"));
        assert!(config("650").contains_key("net0"));

        let gone = client
            .send(Method::DELETE, &path, Some(&cookie), None)
            .await;
        assert_eq!(gone.status, StatusCode::NO_CONTENT);
    }
}
