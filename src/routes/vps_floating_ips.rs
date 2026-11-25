use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

use super::shop::{self, FloatingIpProduct, MAX_FLOATING_IPS};
use super::vps::machine_of;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, floating_ips};
use crate::routes::vps_access::VpsActor;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct FloatingIp {
    id: Uuid,
    address: Option<String>,
    status: String,
    integration_id: Option<Uuid>,
    server_id: Option<Uuid>,
    server_name: Option<String>,
    subscription_code: Option<String>,
    cancelled: bool,
    guest_setup: bool,
    #[serde(serialize_with = "crate::error::public_error")]
    last_error: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct Overview {
    floating_ips: Vec<FloatingIp>,
    used: i64,
    limit: i64,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ServerOption {
    server_id: Uuid,
    server_name: String,
    integration_id: Uuid,
}

#[derive(Serialize)]
pub struct Options {
    products: Vec<FloatingIpProduct>,
    servers: Vec<ServerOption>,
}

#[derive(Deserialize)]
pub struct Attach {
    server_id: Uuid,
}

pub async fn index(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Overview>> {
    actor.require("floating_ips.view")?;
    let floating_ips: Vec<FloatingIp> = sqlx::query_as(
        "SELECT f.id, f.details->>'ipv4' AS address, f.status, f.integration_id,
                s.id AS server_id, s.name AS server_name, sub.code AS subscription_code,
                sub.cancelled_at IS NOT NULL AS cancelled,
                COALESCE(i.kind = 'proxmox', FALSE) AS guest_setup, f.last_error, f.created_at
         FROM services f
         LEFT JOIN services s ON s.id = (f.details->>'server_id')::uuid AND s.status <> 'terminated'
         LEFT JOIN subscriptions sub ON sub.id = f.subscription_id
         LEFT JOIN integrations i ON i.id = f.integration_id
         WHERE f.user_id = $1 AND f.kind = 'floating_ip' AND f.status <> 'terminated'
         ORDER BY f.created_at, f.id",
    )
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    let used = i64::try_from(floating_ips.len()).unwrap_or(i64::MAX);
    Ok(Json(Overview {
        floating_ips,
        used,
        limit: MAX_FLOATING_IPS,
    }))
}

pub async fn options(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Options>> {
    actor.require("floating_ips.view")?;
    let products = if actor.is_owner() {
        shop::floating_ip_products(&state).await?
    } else {
        Vec::new()
    };
    let candidates: Vec<ServerOption> = sqlx::query_as(
        "SELECT id AS server_id, name AS server_name, integration_id FROM services
         WHERE user_id = $1 AND kind = 'vps' AND status = 'active' AND integration_id IS NOT NULL
         ORDER BY name, id",
    )
    .bind(actor.owner)
    .fetch_all(&state.db)
    .await?;
    let mut servers = Vec::new();
    for option in candidates {
        if let Ok(machine) = machine_of(&state, actor.owner, option.server_id).await
            && machine.features().floating_ips
        {
            servers.push(option);
        }
    }
    Ok(Json(Options { products, servers }))
}

struct Owned {
    integration_id: Uuid,
    ip: floating_ips::FloatingIp,
    details: Value,
}

async fn owned(state: &AppState, actor: &VpsActor, id: Uuid) -> AppResult<Owned> {
    let row: (Option<Uuid>, Option<String>, Value, String) = sqlx::query_as(
        "SELECT integration_id, external_id, details, status FROM services
         WHERE id = $1 AND user_id = $2 AND kind = 'floating_ip' AND status <> 'terminated'",
    )
    .bind(id)
    .bind(actor.owner)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;
    let not_ready = || AppError::field("floating_ip", "Deze floating IP is nog niet klaar.");
    if row.3 != "active" {
        return Err(not_ready());
    }
    let address = row.2["ipv4"].as_str().ok_or_else(not_ready)?.to_owned();
    Ok(Owned {
        integration_id: row.0.ok_or_else(not_ready)?,
        ip: floating_ips::FloatingIp {
            id: row.1.ok_or_else(not_ready)?,
            address,
        },
        details: row.2,
    })
}

async fn addresses(state: &AppState, integration_id: Uuid) -> AppResult<floating_ips::Addresses> {
    floating_ips::Addresses::from_driver(integrations::driver(state, integration_id).await?)
        .ok_or_else(|| {
            AppError::field("floating_ip", "Dit platform kan geen floating IPs beheren.")
        })
}

async fn server_external(state: &AppState, server_id: Option<&str>) -> AppResult<Option<String>> {
    let Some(server_id) = server_id.and_then(|id| Uuid::parse_str(id).ok()) else {
        return Ok(None);
    };
    Ok(sqlx::query_scalar(
        "SELECT external_id FROM services WHERE id = $1 AND kind = 'vps' AND status <> 'terminated'",
    )
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .flatten())
}

fn failed(error: anyhow::Error) -> AppError {
    AppError::upstream("floating_ip", error)
}

pub async fn detach(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    actor.require("floating_ips.detach")?;
    let _guard = crate::locks::service(&state.db, id).await?;
    crate::locks::exclusive(format!("floating-ip {id}"), async {
        let floating = owned(&state, &actor, id).await?;
        let Some(server_id) = floating.details["server_id"].as_str() else {
            return Err(AppError::field(
                "floating_ip",
                "Deze floating IP is niet aan een server gekoppeld.",
            ));
        };
        if let Some(server) = server_external(&state, Some(server_id)).await? {
            addresses(&state, floating.integration_id)
                .await?
                .detach(&floating.ip, &server)
                .await
                .map_err(failed)?;
        }
        sqlx::query(
            "UPDATE services SET details = details - 'server_id', updated_at = now() WHERE id = $1",
        )
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
    actor.require("floating_ips.attach")?;
    let _guard = crate::locks::service(&state.db, id).await?;
    crate::locks::exclusive(format!("floating-ip {id}"), async {
        let floating = owned(&state, &actor, id).await?;
        let current = floating.details["server_id"].as_str();
        if current == Some(input.server_id.to_string().as_str()) {
            return Ok(StatusCode::NO_CONTENT);
        }
        if current.is_some() {
            actor.require("floating_ips.detach")?;
        }
        let target: Option<(Option<Uuid>, Option<String>)> = sqlx::query_as(
            "SELECT integration_id, external_id FROM services
             WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status = 'active'",
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
        if integration_id != floating.integration_id {
            return Err(AppError::field(
                "server_id",
                "Deze floating IP kan niet naar deze server. Kies een server op hetzelfde platform.",
            ));
        }
        let addresses = addresses(&state, floating.integration_id).await?;
        if let Some(previous) = server_external(&state, current).await? {
            addresses
                .detach(&floating.ip, &previous)
                .await
                .map_err(failed)?;
            sqlx::query(
                "UPDATE services SET details = details - 'server_id', updated_at = now() WHERE id = $1",
            )
            .bind(id)
            .execute(&state.db)
            .await?;
        }
        addresses
            .attach(&floating.ip, &server)
            .await
            .map_err(failed)?;
        sqlx::query(
            "UPDATE services SET details = details || jsonb_build_object('server_id', $2::text),
                    updated_at = now()
             WHERE id = $1",
        )
        .bind(id)
        .bind(input.server_id.to_string())
        .execute(&state.db)
        .await?;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use axum::http::{Method, StatusCode};
    use serde_json::{Value, json};
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::testing::{Client, NewService, fake_api, make_admin, proxmox_integration, service};

    static IPSETS: Mutex<BTreeMap<String, Vec<String>>> = Mutex::new(BTreeMap::new());

    fn node(method: &str, path: &str, body: &str) -> (u16, Value) {
        let Some((vm, rest)) = path
            .strip_prefix("/api2/json/nodes/pve/qemu/")
            .and_then(|path| path.split_once("/firewall/ipset/ipfilter-net0"))
        else {
            return (200, json!({ "data": null }));
        };
        let mut sets = IPSETS.lock().unwrap();
        let set = sets.entry(vm.to_owned()).or_default();
        match method {
            "POST" => {
                if let Some(cidr) = body.strip_prefix("cidr=") {
                    set.push(cidr.to_owned());
                }
            }
            "DELETE" => set.retain(|cidr| format!("/{cidr}") != rest),
            _ => {}
        }
        let listed: Vec<Value> = set.iter().map(|cidr| json!({ "cidr": cidr })).collect();
        (200, json!({ "data": listed }))
    }

    fn ipset(vm: &str) -> Vec<String> {
        IPSETS.lock().unwrap().get(vm).cloned().unwrap_or_default()
    }

    async fn product(client: &Client, integration: Uuid) -> Uuid {
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO products (id, kind, slug, name, monthly_price_cents, integration_id, specs, provisioning)
             VALUES ($1, 'floating_ip', $1::text, 'Floating IP', 300, $2, '{}', '{}')",
        )
        .bind(id)
        .bind(integration)
        .execute(&client.state.db)
        .await
        .unwrap();
        id
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn an_order_makes_one_subscription_per_address_on_one_invoice(db: PgPool) {
        let client = Client::new(db);
        let (base, _) = fake_api(node).await;
        let integration = proxmox_integration(&client.state, &base).await;
        let product = product(&client, integration).await;
        let (user, cookie) = client.register("fip-order@example.test").await;

        let order = |quantity: u32| json!({ "product_id": product, "quantity": quantity, "billing_interval": "monthly" });
        let placed = client
            .send(
                Method::POST,
                "/vps/floating-ips/order",
                Some(&cookie),
                Some(order(3)),
            )
            .await;
        assert_eq!(placed.status, StatusCode::CREATED, "{}", placed.body);
        assert_eq!(placed.body["service_ids"].as_array().unwrap().len(), 3);

        let (services, subscriptions): (i64, i64) = sqlx::query_as(
            "SELECT count(*), count(DISTINCT subscription_id) FROM services
             WHERE user_id = $1 AND kind = 'floating_ip' AND status = 'pending'",
        )
        .bind(user)
        .fetch_one(&client.state.db)
        .await
        .unwrap();
        assert_eq!((services, subscriptions), (3, 3));
        let (lines, total): (i64, i64) = sqlx::query_as(
            "SELECT count(*), sum(amount_cents)::bigint FROM invoice_lines WHERE invoice_id = $1",
        )
        .bind(placed.body["invoice_id"].as_i64().unwrap())
        .fetch_one(&client.state.db)
        .await
        .unwrap();
        assert_eq!((lines, total), (3, 900));

        let too_many = client
            .send(
                Method::POST,
                "/vps/floating-ips/order",
                Some(&cookie),
                Some(order(8)),
            )
            .await;
        assert_eq!(too_many.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(too_many.body["errors"]["quantity"].is_string());
        let listed = client
            .send(Method::GET, "/vps/floating-ips", Some(&cookie), None)
            .await;
        assert_eq!(listed.body["used"], 3);
        assert_eq!(listed.body["limit"], 10);

        let around = client
            .send(
                Method::POST,
                "/orders",
                Some(&cookie),
                Some(json!({ "product_id": product, "billing_interval": "monthly", "name": "extra" })),
            )
            .await;
        assert_eq!(around.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(around.body["errors"]["product_id"].is_string());
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn a_proxmox_address_moves_between_servers_through_the_ip_filter(db: PgPool) {
        let client = Client::new(db);
        let (base, _) = fake_api(node).await;
        let integration = proxmox_integration(&client.state, &base).await;
        let (user, cookie) = client.register("fip-move@example.test").await;
        let server = |vmid: &'static str, ipv4: &'static str| NewService {
            user,
            subscription: None,
            kind: "vps",
            status: "active",
            integration: Some(integration),
            external_id: Some(vmid),
            details: json!({ "ipv4": ipv4, "vmid": vmid.parse::<u32>().unwrap() }),
        };
        let first = service(&client.state, server("610", "192.0.2.10")).await;
        let second = service(&client.state, server("611", "192.0.2.11")).await;
        let floating = service(
            &client.state,
            NewService {
                user,
                subscription: None,
                kind: "floating_ip",
                status: "pending",
                integration: Some(integration),
                external_id: None,
                details: json!({}),
            },
        )
        .await;

        crate::services::provision_in_background(client.state.clone(), floating);
        let mut row: (String, Option<String>, String) = Default::default();
        for _ in 0..50 {
            row = sqlx::query_as("SELECT status, external_id, name FROM services WHERE id = $1")
                .bind(floating)
                .fetch_one(&client.state.db)
                .await
                .unwrap();
            if row.0 != "pending" && row.0 != "provisioning" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        assert_eq!(
            row,
            (
                "active".to_owned(),
                Some("192.0.2.12".to_owned()),
                "192.0.2.12".to_owned()
            )
        );

        let attach = |server: Uuid| json!({ "server_id": server });
        let path = format!("/vps/floating-ips/{floating}");
        let reply = client
            .send(
                Method::POST,
                &format!("{path}/attach"),
                Some(&cookie),
                Some(attach(first)),
            )
            .await;
        assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
        assert_eq!(ipset("610"), ["192.0.2.12"]);

        let reply = client
            .send(
                Method::POST,
                &format!("{path}/attach"),
                Some(&cookie),
                Some(attach(second)),
            )
            .await;
        assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
        assert!(ipset("610").is_empty());
        assert_eq!(ipset("611"), ["192.0.2.12"]);

        let helper = crate::testing::shared_member(
            &client,
            &cookie,
            user,
            "fip-helper@example.test",
            &["floating_ips.view", "floating_ips.attach"],
        )
        .await;
        let moved = client
            .send_with(
                Method::POST,
                &format!("{path}/attach"),
                Some(&helper),
                Some(attach(first)),
                &[("x-vps-owner", &user.to_string())],
            )
            .await;
        assert_eq!(moved.status, StatusCode::FORBIDDEN);
        assert_eq!(ipset("611"), ["192.0.2.12"]);

        let listed = client
            .send(Method::GET, "/vps/floating-ips", Some(&cookie), None)
            .await;
        let row = &listed.body["floating_ips"][0];
        assert_eq!(row["server_id"], second.to_string());
        assert_eq!(row["address"], "192.0.2.12");
        assert_eq!(row["guest_setup"], true);

        let reply = client
            .send(Method::POST, &format!("{path}/detach"), Some(&cookie), None)
            .await;
        assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
        assert!(ipset("611").is_empty());

        let (_, other_cookie) = client.register("fip-other@example.test").await;
        let reply = client
            .send(
                Method::POST,
                &format!("{path}/attach"),
                Some(&other_cookie),
                Some(attach(first)),
            )
            .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn suspending_takes_the_address_off_its_server_until_it_is_back(db: PgPool) {
        let client = Client::new(db);
        let (base, _) = fake_api(node).await;
        let integration = proxmox_integration(&client.state, &base).await;
        let (user, _) = client.register("fip-suspend@example.test").await;
        let (admin, admin_cookie) = client.register("fip-staff@example.test").await;
        make_admin(&client.state, admin).await;
        let server = service(
            &client.state,
            NewService {
                user,
                subscription: None,
                kind: "vps",
                status: "active",
                integration: Some(integration),
                external_id: Some("620"),
                details: json!({ "ipv4": "192.0.2.10" }),
            },
        )
        .await;
        IPSETS
            .lock()
            .unwrap()
            .insert("620".to_owned(), vec!["192.0.2.15".to_owned()]);
        let floating = service(
            &client.state,
            NewService {
                user,
                subscription: None,
                kind: "floating_ip",
                status: "active",
                integration: Some(integration),
                external_id: Some("192.0.2.15"),
                details: json!({ "ipv4": "192.0.2.15", "server_id": server.to_string() }),
            },
        )
        .await;
        let act = |action: &str| {
            let path = format!("/admin/services/{floating}/{action}");
            let cookie = admin_cookie.clone();
            let client = &client;
            async move {
                let reply = client.send(Method::POST, &path, Some(&cookie), None).await;
                assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.body);
            }
        };
        let details = || async {
            sqlx::query_scalar::<_, Value>("SELECT details FROM services WHERE id = $1")
                .bind(floating)
                .fetch_one(&client.state.db)
                .await
                .unwrap()
        };

        act("suspend").await;
        assert!(ipset("620").is_empty());
        assert_eq!(details().await["suspended_from"], server.to_string());

        act("unsuspend").await;
        assert_eq!(ipset("620"), ["192.0.2.15"]);
        assert_eq!(details().await["server_id"], server.to_string());

        act("terminate").await;
        assert!(ipset("620").is_empty());
    }
}
