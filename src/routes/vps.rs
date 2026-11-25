use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::vps_access::VpsActor;
use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, vps};
use crate::payments::reveal;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Server {
    id: Uuid,
    name: String,
    status: String,
    details: Value,
    #[serde(serialize_with = "crate::error::public_error")]
    last_error: Option<String>,
    product_name: Option<String>,
    subscription_code: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct ServerDetail {
    #[serde(flatten)]
    server: Server,
    live: Option<vps::Status>,
    features: Option<vps::Features>,
}

const SELECT: &str = "SELECT s.id, s.name, s.status, s.details - 'root_password' - 'ssh_key_ids' AS details, s.last_error,
        p.name AS product_name, sub.code AS subscription_code, sub.expires_at, s.created_at
     FROM services s
     LEFT JOIN products p ON p.id = s.product_id
     LEFT JOIN subscriptions sub ON sub.id = s.subscription_id
     WHERE s.kind = 'vps' AND s.status <> 'terminated' AND s.user_id = $1";

async fn load(state: &AppState, owner: Uuid, id: Uuid) -> AppResult<Server> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND s.id = $2")))
        .bind(owner)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)
}

async fn machine(state: &AppState, owner: Uuid, id: Uuid) -> AppResult<vps::Machine> {
    machine_of(state, owner, id).await
}

pub(crate) async fn machine_of(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
) -> AppResult<vps::Machine> {
    connect(state, user_id, id, false).await
}

async fn connect(
    state: &AppState,
    user_id: Uuid,
    id: Uuid,
    while_rebuilding: bool,
) -> AppResult<vps::Machine> {
    let (integration_id, external_id, status, rebuilding): (
        Option<Uuid>,
        Option<String>,
        String,
        bool,
    ) = sqlx::query_as(
        "SELECT integration_id, external_id, status, details ? 'rebuilding' FROM services
         WHERE id = $1 AND user_id = $2 AND kind = 'vps'",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    if rebuilding && !while_rebuilding {
        return Err(AppError::field(
            "server",
            "Deze server wordt herbouwd. Wacht tot dat klaar is.",
        ));
    }
    if status != "active" {
        return Err(AppError::field(
            "server",
            "Deze server is nu niet beschikbaar.",
        ));
    }
    let not_ready = || AppError::field("server", "Deze server is nog niet klaar.");
    let (Some(integration_id), Some(external_id)) = (integration_id, external_id) else {
        return Err(not_ready());
    };
    match integrations::driver(state, integration_id).await? {
        integrations::Driver::Proxmox(client) => Ok(vps::Machine::Proxmox(
            client,
            external_id.parse().map_err(|_| not_ready())?,
        )),
        integrations::Driver::OpenStack(client) => Ok(vps::Machine::OpenStack(client, external_id)),
        integrations::Driver::VirtFusion(client) => {
            Ok(vps::Machine::VirtFusion(client, external_id))
        }
        _ => Err(AppError::NotFound),
    }
}

fn failed(error: anyhow::Error) -> AppError {
    AppError::upstream("server", error)
}

pub async fn index(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Vec<Server>>> {
    actor.require("servers.view")?;
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{SELECT} ORDER BY s.created_at DESC"
        )))
        .bind(actor.owner)
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn show(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ServerDetail>> {
    actor.require("servers.view")?;
    let server = load(&state, actor.owner, id).await?;
    let (live, features) = match connect(&state, actor.owner, id, true).await {
        Ok(machine) => (machine.status().await.ok(), Some(machine.features())),
        Err(_) => (None, None),
    };
    Ok(Json(ServerDetail {
        server,
        live,
        features,
    }))
}

#[derive(Deserialize)]
pub struct Power {
    action: String,
}

pub async fn power(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<Power>,
) -> AppResult<StatusCode> {
    if !matches!(
        input.action.as_str(),
        "start" | "shutdown" | "stop" | "reboot"
    ) {
        return Err(AppError::field("action", "Onbekende actie."));
    }
    if input.action != "stop" && input.action != "shutdown" {
        actor.require("servers.start")?;
    }
    if input.action != "start" {
        actor.require("servers.stop")?;
    }
    let machine = machine(&state, actor.owner, id).await?;
    machine.power(&input.action).await.map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct ImageChoice {
    id: String,
    name: String,
}

pub async fn images(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<ImageChoice>>> {
    actor.require("servers.rebuild")?;
    let integration_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT integration_id FROM services WHERE id = $1 AND user_id = $2 AND kind = 'vps'",
    )
    .bind(id)
    .bind(actor.owner)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let Some(integration_id) = integration_id else {
        return Ok(Json(Vec::new()));
    };
    let choices = match integrations::driver(&state, integration_id).await? {
        integrations::Driver::Proxmox(client) => client
            .settings()
            .images
            .iter()
            .map(|image| ImageChoice {
                id: image.template.to_string(),
                name: image.name.clone(),
            })
            .collect(),
        integrations::Driver::OpenStack(client) => client
            .settings()
            .images
            .iter()
            .map(|image| ImageChoice {
                id: image.id.clone(),
                name: image.name.clone(),
            })
            .collect(),
        _ => Vec::new(),
    };
    Ok(Json(choices))
}

#[derive(Deserialize)]
pub struct RebuildInput {
    image: String,
    #[serde(default)]
    ssh_key_ids: Vec<Uuid>,
}

pub async fn rebuild(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<RebuildInput>,
) -> AppResult<StatusCode> {
    actor.require("servers.rebuild")?;
    if !actor.is_owner() && !input.ssh_key_ids.is_empty() {
        actor.require("ssh_keys.view")?;
    }
    crate::services::start_rebuild(
        &state,
        id,
        actor.owner,
        crate::services::RebuildRequest {
            image: input.image,
            ssh_key_ids: input.ssh_key_ids,
        },
    )
    .await?;
    Ok(StatusCode::ACCEPTED)
}

#[derive(Serialize)]
pub struct Password {
    password: Option<String>,
}

pub async fn root_password(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Password>> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    let sealed: Option<String> = sqlx::query_scalar(
        "SELECT details->>'root_password' FROM services WHERE id = $1 AND user_id = $2 AND kind = 'vps'",
    )
    .bind(id)
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    Ok(Json(Password {
        password: reveal(&state, sealed.as_ref()),
    }))
}

#[derive(Serialize)]
pub struct PanelLink {
    url: String,
}

pub async fn panel_login(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PanelLink>> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    match machine(&state, user.id, id).await? {
        vps::Machine::VirtFusion(client, server) => Ok(Json(PanelLink {
            url: client
                .login_url(&user.id.to_string(), &server)
                .await
                .map_err(failed)?,
        })),
        _ => Err(AppError::NotFound),
    }
}

pub async fn snapshots(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<vps::Snapshot>>> {
    actor.require("snapshots.view")?;
    let machine = machine(&state, actor.owner, id).await?;
    Ok(Json(machine.snapshots().await.map_err(failed)?))
}

#[derive(Serialize)]
pub struct ServerSnapshot {
    server_id: Uuid,
    server_name: String,
    #[serde(flatten)]
    snapshot: vps::Snapshot,
}

#[derive(Serialize)]
pub struct SnapshotOverview {
    snapshots: Vec<ServerSnapshot>,
    used: usize,
    limit: usize,
}

pub async fn all_snapshots(
    State(state): State<AppState>,
    actor: VpsActor,
) -> AppResult<Json<SnapshotOverview>> {
    actor.require("snapshots.view")?;
    let servers: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, name FROM services WHERE kind = 'vps' AND status = 'active' AND user_id = $1 ORDER BY name",
    )
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    let mut snapshots = Vec::new();
    let mut capable = 0;
    for (server_id, server_name) in &servers {
        let Ok(machine) = machine(&state, actor.owner, *server_id).await else {
            continue;
        };
        if !machine.features().snapshots {
            continue;
        }
        capable += 1;
        let Ok(found) = machine.snapshots().await else {
            continue;
        };
        snapshots.extend(found.into_iter().map(|snapshot| ServerSnapshot {
            server_id: *server_id,
            server_name: server_name.clone(),
            snapshot,
        }));
    }
    snapshots.sort_by_key(|item| std::cmp::Reverse(item.snapshot.created_at));
    Ok(Json(SnapshotOverview {
        used: snapshots.len(),
        limit: capable * MAX_SNAPSHOTS,
        snapshots,
    }))
}

#[derive(Deserialize)]
pub struct NewSnapshot {
    description: Option<String>,
}

const MAX_SNAPSHOTS: usize = 3;

pub async fn create_snapshot(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<NewSnapshot>,
) -> AppResult<StatusCode> {
    actor.require("snapshots.create")?;
    let key = format!("snapshots {id}");
    crate::locks::exclusive(key, take_snapshot(&state, &actor, id, input)).await
}

async fn take_snapshot(
    state: &AppState,
    actor: &VpsActor,
    id: Uuid,
    input: NewSnapshot,
) -> AppResult<StatusCode> {
    let machine = machine(state, actor.owner, id).await?;
    if machine.snapshots().await.map_err(failed)?.len() >= MAX_SNAPSHOTS {
        return Err(AppError::field(
            "description",
            "Je hebt al 3 snapshots. Verwijder er eerst een.",
        ));
    }
    let name = OffsetDateTime::now_utc()
        .format(time::macros::format_description!(
            "snap[year][month][day][hour][minute][second]"
        ))
        .map_err(anyhow::Error::from)?;
    let description = input.description.unwrap_or_default();
    machine
        .create_snapshot(&name, description.trim())
        .await
        .map_err(failed)?;
    Ok(StatusCode::CREATED)
}

fn identifier(name: &str) -> AppResult<&str> {
    if name.is_empty()
        || name.len() > 40
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '_' || character == '-'
        })
    {
        return Err(AppError::NotFound);
    }
    Ok(name)
}

pub async fn rollback_snapshot(
    State(state): State<AppState>,
    actor: VpsActor,
    Path((id, name)): Path<(Uuid, String)>,
) -> AppResult<StatusCode> {
    actor.require("servers.restore")?;
    let machine = machine(&state, actor.owner, id).await?;
    machine
        .rollback_snapshot(identifier(&name)?)
        .await
        .map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_snapshot(
    State(state): State<AppState>,
    actor: VpsActor,
    Path((id, name)): Path<(Uuid, String)>,
) -> AppResult<StatusCode> {
    actor.require("snapshots.delete")?;
    let machine = machine(&state, actor.owner, id).await?;
    machine
        .delete_snapshot(identifier(&name)?)
        .await
        .map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn firewall(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<vps::FirewallRule>>> {
    actor.require("firewalls.view")?;
    let machine = machine(&state, actor.owner, id).await?;
    Ok(Json(machine.firewall_rules().await.map_err(failed)?))
}

pub const MAX_PORTS_PER_RULE: usize = 15;
const MAX_RULES_PER_SERVER: usize = 100;

fn valid_ports(ports: &str) -> bool {
    ports.split(',').count() <= MAX_PORTS_PER_RULE
        && ports.split(',').all(|part| {
            let mut bounds = part.split(':');
            let ok = |value: Option<&str>| {
                value.is_some_and(|value| value.parse::<u16>().is_ok_and(|port| port > 0))
            };
            match (bounds.next(), bounds.next(), bounds.next()) {
                (first, None, None) => ok(first),
                (first, second, None) => ok(first) && ok(second),
                _ => false,
            }
        })
}

fn valid_source(source: &str) -> bool {
    let (address, prefix) = source.split_once('/').unwrap_or((source, "32"));
    address.parse::<std::net::Ipv4Addr>().is_ok()
        && prefix.parse::<u8>().is_ok_and(|prefix| prefix <= 32)
}

pub fn validate_rule(rule: &mut vps::FirewallRule) -> AppResult<()> {
    rule.direction = rule.direction.to_lowercase();
    rule.action = rule.action.to_uppercase();
    if !matches!(rule.direction.as_str(), "in" | "out")
        || !matches!(rule.action.as_str(), "ACCEPT" | "DROP" | "REJECT")
    {
        return Err(AppError::field(
            "action",
            "Kies een richting en of het verkeer mag.",
        ));
    }
    if rule
        .proto
        .as_deref()
        .is_some_and(|proto| !proto.is_empty() && !matches!(proto, "tcp" | "udp" | "icmp"))
    {
        return Err(AppError::field("proto", "Kies TCP, UDP of ICMP."));
    }
    if rule
        .dport
        .as_deref()
        .is_some_and(|ports| !ports.is_empty() && !valid_ports(ports))
    {
        return Err(AppError::field(
            "dport",
            "Vul hoogstens 15 poorten of reeksen in, zoals 22, 80,443 of 8000:8100.",
        ));
    }
    rule.comment = rule
        .comment
        .as_deref()
        .map(|comment| comment.trim().chars().take(80).collect::<String>())
        .filter(|comment| !comment.is_empty());
    if rule
        .source
        .as_deref()
        .is_some_and(|source| !source.is_empty() && !valid_source(source))
    {
        return Err(AppError::field(
            "source",
            "Vul een IPv4-adres of -reeks in, zoals 203.0.113.4 of 10.0.0.0/8.",
        ));
    }
    Ok(())
}

pub async fn add_firewall_rule(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(mut rule): Json<vps::FirewallRule>,
) -> AppResult<StatusCode> {
    actor.require("firewalls.rule_create")?;
    validate_rule(&mut rule)?;
    let machine = machine(&state, actor.owner, id).await?;
    if rule.action != "ACCEPT" && !machine.can_block() {
        return Err(AppError::field(
            "action",
            "Op dit platform kun je alleen verkeer toestaan.",
        ));
    }
    if !machine.can_block()
        && rule
            .dport
            .as_deref()
            .is_some_and(|ports| ports.contains(','))
    {
        return Err(AppError::field(
            "dport",
            "Op dit platform geef je per regel één poort of reeks op.",
        ));
    }
    if machine.firewall_rules().await.map_err(failed)?.len() >= MAX_RULES_PER_SERVER {
        return Err(AppError::field(
            "dport",
            "Deze server heeft al 100 firewallregels. Verwijder er eerst een paar.",
        ));
    }
    machine.add_firewall_rule(&rule).await.map_err(failed)?;
    Ok(StatusCode::CREATED)
}

pub async fn delete_firewall_rule(
    State(state): State<AppState>,
    actor: VpsActor,
    Path((id, rule)): Path<(Uuid, String)>,
) -> AppResult<StatusCode> {
    actor.require("firewalls.rule_delete")?;
    let machine = machine(&state, actor.owner, id).await?;
    machine
        .delete_firewall_rule(identifier(&rule)?)
        .await
        .map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::{valid_ports, valid_source};

    #[test]
    fn checks_ports_and_sources() {
        assert!(valid_ports("22"));
        assert!(valid_ports("80,443"));
        assert!(valid_ports("8000:8100"));
        assert!(!valid_ports("0"));
        assert!(!valid_ports("ssh"));
        assert!(!valid_ports("1:2:3"));
        assert!(valid_ports(
            &(1..=15)
                .map(|port| port.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        assert!(!valid_ports(
            &(1..=16)
                .map(|port| port.to_string())
                .collect::<Vec<_>>()
                .join(",")
        ));
        assert!(valid_source("203.0.113.4"));
        assert!(valid_source("10.0.0.0/8"));
        assert!(!valid_source("10.0.0.0/40"));
        assert!(!valid_source("example.com"));
    }
}

#[cfg(test)]
mod rebuilds {
    use axum::http::{Method, StatusCode};
    use serde_json::json;
    use sqlx::PgPool;

    use crate::testing::{Client, NewService, fake_api, proxmox_integration, service};

    fn node(_: &str, _: &str, _: &str) -> (u16, serde_json::Value) {
        (200, json!({ "data": { "status": "running" } }))
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn a_server_being_rebuilt_takes_no_other_actions(db: PgPool) {
        let client = Client::new(db);
        let (base, _) = fake_api(node).await;
        let integration = proxmox_integration(&client.state, &base).await;
        let (user, cookie) = client.register("rebuild@example.test").await;
        let server = service(
            &client.state,
            NewService {
                user,
                subscription: None,
                kind: "vps",
                status: "active",
                integration: Some(integration),
                external_id: Some("710"),
                details: json!({ "rebuilding": true }),
            },
        )
        .await;

        let power = client
            .send(
                Method::POST,
                &format!("/vps/servers/{server}/power"),
                Some(&cookie),
                Some(json!({ "action": "reboot" })),
            )
            .await;
        assert_eq!(power.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(power.body["errors"]["server"].is_string());

        let again = client
            .send(
                Method::POST,
                &format!("/vps/servers/{server}/rebuild"),
                Some(&cookie),
                Some(json!({ "image": "nonexistent" })),
            )
            .await;
        assert_eq!(again.status, StatusCode::UNPROCESSABLE_ENTITY);

        let shown = client
            .send(
                Method::GET,
                &format!("/vps/servers/{server}"),
                Some(&cookie),
                None,
            )
            .await;
        assert_eq!(shown.status, StatusCode::OK);
        assert_eq!(shown.body["details"]["rebuilding"], true);
        assert_eq!(shown.body["features"]["rebuild"], true);

        let (_, stranger) = client.register("rebuild-other@example.test").await;
        let foreign = client
            .send(
                Method::GET,
                &format!("/vps/servers/{server}/images"),
                Some(&stranger),
                None,
            )
            .await;
        assert_eq!(foreign.status, StatusCode::NOT_FOUND);
    }
}
