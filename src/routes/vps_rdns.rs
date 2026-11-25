use std::net::{IpAddr, Ipv4Addr};

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::integrations::{self, powerdns};
use crate::limits;
use crate::routes::vps_access::VpsActor;
use crate::state::AppState;

#[derive(Serialize)]
pub struct Address {
    ip: String,
    ptr: Option<String>,
    editable: bool,
}

#[derive(Deserialize)]
pub struct Change {
    ip: Ipv4Addr,
    hostname: Option<String>,
}

async fn dns_servers(state: &AppState) -> AppResult<Vec<powerdns::PowerDns>> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM integrations WHERE kind = 'powerdns' ORDER BY created_at",
    )
    .fetch_all(&state.db)
    .await?;
    let mut servers = Vec::new();
    for id in ids {
        if let integrations::Driver::PowerDns(client) = integrations::driver(state, id).await? {
            servers.push(client);
        }
    }
    Ok(servers)
}

async fn locate(
    servers: &[powerdns::PowerDns],
    ip: Ipv4Addr,
) -> Option<(&powerdns::PowerDns, String)> {
    for server in servers {
        match server.zone_of(ip).await {
            Ok(Some(zone)) => return Some((server, zone)),
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%ip, error = %format!("{error:#}"), "looking up a reverse zone failed");
            }
        }
    }
    None
}

async fn addresses_of(state: &AppState, owner: Uuid, id: Uuid) -> AppResult<Vec<Ipv4Addr>> {
    let main: Option<Option<String>> = sqlx::query_scalar(
        "SELECT details->>'ipv4' FROM services
         WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status IN ('active', 'suspended')",
    )
    .bind(id)
    .bind(owner)
    .fetch_optional(&state.db)
    .await?;
    let Some(main) = main else {
        return Err(AppError::NotFound);
    };
    let floating: Vec<String> = sqlx::query_scalar(
        "SELECT details->>'ipv4' FROM services
         WHERE user_id = $1 AND kind = 'floating_ip' AND status = 'active'
           AND details->>'server_id' = $2::text AND details ? 'ipv4'
         ORDER BY created_at",
    )
    .bind(owner)
    .bind(id.to_string())
    .fetch_all(&state.db)
    .await?;
    Ok(main
        .into_iter()
        .chain(floating)
        .filter_map(|ip| ip.parse().ok())
        .collect())
}

pub async fn index(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<Address>>> {
    actor.require("servers.view")?;
    let ips = addresses_of(&state, actor.owner, id).await?;
    let servers = dns_servers(&state).await?;
    let mut addresses = Vec::new();
    for ip in ips {
        let (ptr, editable) = match locate(&servers, ip).await {
            Some((server, zone)) => (server.ptr(&zone, ip).await.ok().flatten(), true),
            None => (None, false),
        };
        addresses.push(Address {
            ip: ip.to_string(),
            ptr,
            editable,
        });
    }
    Ok(Json(addresses))
}

async fn points_to(hostname: &str, ip: Ipv4Addr) -> bool {
    let Ok(found) = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::net::lookup_host(format!("{hostname}:0")),
    )
    .await
    else {
        return false;
    };
    found.is_ok_and(|mut addresses| addresses.any(|address| address.ip() == IpAddr::V4(ip)))
}

pub async fn update(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<Change>,
) -> AppResult<StatusCode> {
    actor.require("servers.rdns")?;
    state
        .limits
        .check(&limits::RDNS, &actor.owner.to_string())?;
    if !addresses_of(&state, actor.owner, id)
        .await?
        .contains(&input.ip)
    {
        return Err(AppError::field(
            "ip",
            "Dit IP-adres hoort niet bij deze server.",
        ));
    }
    let hostname = input
        .hostname
        .as_deref()
        .map(|name| name.trim().trim_end_matches('.').to_ascii_lowercase())
        .filter(|name| !name.is_empty());
    if let Some(hostname) = &hostname {
        if !powerdns::valid_hostname(hostname) {
            return Err(AppError::field(
                "hostname",
                "Vul een volledige hostnaam in, zoals mail.jouwdomein.nl.",
            ));
        }
        if !points_to(hostname, input.ip).await {
            return Err(AppError::field(
                "hostname",
                "Laat deze hostnaam eerst met een A-record naar dit IP-adres wijzen.",
            ));
        }
    }
    sqlx::query("DELETE FROM ptr_cleanups WHERE ip = $1")
        .bind(input.ip.to_string())
        .execute(&state.db)
        .await?;
    let servers = dns_servers(&state).await?;
    let Some((server, zone)) = locate(&servers, input.ip).await else {
        return Err(AppError::field(
            "ip",
            "Voor dit IP-adres kun je hier geen reverse DNS instellen.",
        ));
    };
    server
        .set_ptr(&zone, input.ip, hostname.as_deref())
        .await
        .map_err(|error| AppError::upstream("hostname", error))?;
    Ok(StatusCode::NO_CONTENT)
}

async fn cleared(state: &AppState, ip: Ipv4Addr) -> bool {
    let servers = match dns_servers(state).await {
        Ok(servers) => servers,
        Err(error) => {
            tracing::warn!(%ip, ?error, "loading dns servers failed");
            return false;
        }
    };
    for server in &servers {
        match server.zone_of(ip).await {
            Ok(Some(zone)) => {
                return match server.set_ptr(&zone, ip, None).await {
                    Ok(()) => true,
                    Err(error) => {
                        tracing::warn!(%ip, error = %format!("{error:#}"), "clearing a ptr record failed");
                        false
                    }
                };
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%ip, error = %format!("{error:#}"), "looking up a reverse zone failed");
                return false;
            }
        }
    }
    true
}

pub async fn queue(
    connection: &mut sqlx::PgConnection,
    ip: &str,
    service_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO ptr_cleanups (ip, service_id) VALUES ($1, $2)
         ON CONFLICT (ip) DO UPDATE SET service_id = EXCLUDED.service_id",
    )
    .bind(ip)
    .bind(service_id)
    .execute(connection)
    .await?;
    Ok(())
}

pub async fn clear(state: &AppState, ip: &str) {
    let Ok(address) = ip.parse::<Ipv4Addr>() else {
        return;
    };
    if cleared(state, address).await
        && let Err(error) = sqlx::query("DELETE FROM ptr_cleanups WHERE ip = $1")
            .bind(ip)
            .execute(&state.db)
            .await
    {
        tracing::warn!(%ip, ?error, "finishing a ptr cleanup failed");
    }
}

pub async fn retry_cleanups(state: &AppState) -> AppResult<usize> {
    let pending: Vec<String> =
        sqlx::query_scalar("SELECT ip FROM ptr_cleanups ORDER BY created_at")
            .fetch_all(&state.db)
            .await?;
    let mut done = 0;
    for ip in pending {
        let Ok(address) = ip.parse::<Ipv4Addr>() else {
            continue;
        };
        let reissued: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM services s, ptr_cleanups p
                            WHERE p.ip = $1 AND s.details->>'ipv4' = $1 AND s.status <> 'terminated'
                              AND s.id IS DISTINCT FROM p.service_id)",
        )
        .bind(&ip)
        .fetch_one(&state.db)
        .await?;
        if reissued || cleared(state, address).await {
            sqlx::query("DELETE FROM ptr_cleanups WHERE ip = $1")
                .bind(&ip)
                .execute(&state.db)
                .await?;
            done += 1;
        } else {
            sqlx::query("UPDATE ptr_cleanups SET attempts = attempts + 1 WHERE ip = $1")
                .bind(&ip)
                .execute(&state.db)
                .await?;
        }
    }
    Ok(done)
}

#[cfg(test)]
mod tests {
    use axum::http::{Method, StatusCode};
    use serde_json::{Value, json};
    use sqlx::PgPool;
    use uuid::Uuid;

    use crate::testing::{Client, NewService, fake_api, service};

    fn dns(method: &str, path: &str, _: &str) -> (u16, Value) {
        match (method, path) {
            ("GET", "/api/v1/servers/localhost/zones") => {
                (200, json!([{ "name": "2.0.192.in-addr.arpa." }]))
            }
            ("GET", path) if path.contains("rrset_name=15.2.0.192.in-addr.arpa.") => (
                200,
                json!({ "rrsets": [{ "name": "15.2.0.192.in-addr.arpa.", "type": "PTR", "records": [{ "content": "web.example.com." }] }] }),
            ),
            _ => (200, json!({})),
        }
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn customers_manage_ptr_records_of_their_own_addresses(db: PgPool) {
        let client = Client::new(db);
        let (base, calls) = fake_api(dns).await;
        let sealed =
            crate::integrations::seal_credentials(&client.state, &json!({ "api_key": "k" }))
                .unwrap();
        sqlx::query(
            "INSERT INTO integrations (id, kind, name, base_url, credentials, settings)
             VALUES ($1, 'powerdns', 'DNS', $2, $3, '{\"server_id\": \"localhost\"}')",
        )
        .bind(Uuid::new_v4())
        .bind(&base)
        .bind(sealed)
        .execute(&client.state.db)
        .await
        .unwrap();
        let (user, cookie) = client.register("rdns@example.test").await;
        let server = service(
            &client.state,
            NewService {
                user,
                subscription: None,
                kind: "vps",
                status: "active",
                integration: None,
                external_id: Some("1"),
                details: json!({ "ipv4": "192.0.2.15" }),
            },
        )
        .await;
        let path = format!("/vps/servers/{server}/rdns");

        let listed = client.send(Method::GET, &path, Some(&cookie), None).await;
        assert_eq!(listed.status, StatusCode::OK, "{}", listed.body);
        assert_eq!(
            listed.body,
            json!([{ "ip": "192.0.2.15", "ptr": "web.example.com", "editable": true }])
        );

        for (body, field) in [
            (
                json!({ "ip": "192.0.2.16", "hostname": "web.example.com" }),
                "ip",
            ),
            (
                json!({ "ip": "192.0.2.15", "hostname": "not a host" }),
                "hostname",
            ),
            (
                json!({ "ip": "192.0.2.15", "hostname": "nothing-here.invalid" }),
                "hostname",
            ),
        ] {
            let refused = client
                .send(Method::PUT, &path, Some(&cookie), Some(body))
                .await;
            assert_eq!(refused.status, StatusCode::UNPROCESSABLE_ENTITY);
            assert!(
                refused.body["errors"][field].is_string(),
                "{}",
                refused.body
            );
        }

        let cleared = client
            .send(
                Method::PUT,
                &path,
                Some(&cookie),
                Some(json!({ "ip": "192.0.2.15", "hostname": null })),
            )
            .await;
        assert_eq!(cleared.status, StatusCode::NO_CONTENT, "{}", cleared.body);
        assert!(calls.lock().unwrap().iter().any(|call| {
            call.starts_with("PATCH /api/v1/servers/localhost/zones/2.0.192.in-addr.arpa.")
                && call.contains("\"changetype\":\"DELETE\"")
        }));

        let (_, stranger) = client.register("rdns-other@example.test").await;
        let peek = client.send(Method::GET, &path, Some(&stranger), None).await;
        assert_eq!(peek.status, StatusCode::NOT_FOUND);
    }

    static DOWN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

    fn flaky(method: &str, path: &str, body: &str) -> (u16, Value) {
        if DOWN.load(std::sync::atomic::Ordering::SeqCst) {
            return (503, json!({ "error": "down" }));
        }
        dns(method, path, body)
    }

    #[sqlx::test]
    #[ignore = "needs DATABASE_URL"]
    async fn a_ptr_that_could_not_be_cleared_is_retried_until_it_is_gone(db: PgPool) {
        let client = Client::new(db);
        let (base, calls) = fake_api(flaky).await;
        let sealed =
            crate::integrations::seal_credentials(&client.state, &json!({ "api_key": "k" }))
                .unwrap();
        sqlx::query(
            "INSERT INTO integrations (id, kind, name, base_url, credentials, settings)
             VALUES ($1, 'powerdns', 'DNS', $2, $3, '{\"server_id\": \"localhost\"}')",
        )
        .bind(Uuid::new_v4())
        .bind(&base)
        .bind(sealed)
        .execute(&client.state.db)
        .await
        .unwrap();

        let mut connection = client.state.db.acquire().await.unwrap();
        super::queue(&mut connection, "192.0.2.15", Uuid::new_v4())
            .await
            .unwrap();
        drop(connection);
        super::clear(&client.state, "192.0.2.15").await;
        let queued: Vec<String> = sqlx::query_scalar("SELECT ip FROM ptr_cleanups")
            .fetch_all(&client.state.db)
            .await
            .unwrap();
        assert_eq!(queued, ["192.0.2.15"]);

        assert_eq!(super::retry_cleanups(&client.state).await.unwrap(), 0);
        DOWN.store(false, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(super::retry_cleanups(&client.state).await.unwrap(), 1);
        let left: i64 = sqlx::query_scalar("SELECT count(*) FROM ptr_cleanups")
            .fetch_one(&client.state.db)
            .await
            .unwrap();
        assert_eq!(left, 0);
        assert!(calls.lock().unwrap().iter().any(|call| {
            call.starts_with("PATCH /api/v1/servers/localhost/zones/2.0.192.in-addr.arpa.")
                && call.contains("\"changetype\":\"DELETE\"")
        }));
        DOWN.store(true, std::sync::atomic::Ordering::SeqCst);
        let (owner, _) = client.register("rdns-reissued@example.test").await;
        crate::testing::service(
            &client.state,
            crate::testing::NewService {
                user: owner,
                subscription: None,
                kind: "floating_ip",
                status: "active",
                integration: None,
                external_id: None,
                details: json!({ "ipv4": "192.0.2.16" }),
            },
        )
        .await;
        let mut connection = client.state.db.acquire().await.unwrap();
        super::queue(&mut connection, "192.0.2.16", Uuid::new_v4())
            .await
            .unwrap();
        drop(connection);
        let before = calls.lock().unwrap().len();
        assert_eq!(super::retry_cleanups(&client.state).await.unwrap(), 1);
        assert_eq!(calls.lock().unwrap().len(), before);
    }
}
