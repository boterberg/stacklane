use std::collections::HashMap;

use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::Report;
use crate::auth;
use crate::integrations::pterodactyl::Pterodactyl;

pub struct ImportedServer {
    pub external_id: String,
    pub owner_external_id: i64,
    pub name: String,
    pub suspended: bool,
    pub details: Value,
}

fn server_from(value: &Value) -> Option<ImportedServer> {
    let attributes = &value["attributes"];
    let id = attributes["id"].as_i64()?;
    let allocation = attributes["relationships"]["allocations"]["data"]
        .as_array()
        .and_then(|allocations| {
            let primary = attributes["allocation"].as_i64();
            allocations
                .iter()
                .find(|allocation| allocation["attributes"]["id"].as_i64() == primary)
                .or(allocations.first())
        })
        .map(|allocation| &allocation["attributes"]);
    Some(ImportedServer {
        external_id: id.to_string(),
        owner_external_id: attributes["user"].as_i64()?,
        name: attributes["name"]
            .as_str()
            .unwrap_or("Gameserver")
            .to_owned(),
        suspended: attributes["suspended"].as_bool().unwrap_or(false),
        details: json!({
            "identifier": attributes["identifier"],
            "uuid": attributes["uuid"],
            "address": allocation.map(|allocation| format!(
                "{}:{}",
                allocation["alias"].as_str().filter(|alias| !alias.is_empty()).or(allocation["ip"].as_str()).unwrap_or_default(),
                allocation["port"]
            )),
            "memory": attributes["limits"]["memory"],
            "disk": attributes["limits"]["disk"],
            "cpu": attributes["limits"]["cpu"],
            "egg": attributes["relationships"]["egg"]["attributes"]["name"],
            "imported_from": "pterodactyl",
        }),
    })
}

async fn ensure_user(
    pool: &PgPool,
    pterodactyl_user: &Value,
    dry_run: bool,
    report: &mut Report,
) -> anyhow::Result<Option<Uuid>> {
    let attributes = &pterodactyl_user["attributes"];
    let Some(email) = attributes["email"]
        .as_str()
        .map(|email| email.trim().to_lowercase())
    else {
        return Ok(None);
    };
    if let Some(id) = sqlx::query_scalar::<_, Uuid>("SELECT id FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(pool)
        .await?
    {
        report.existing("Klanten");
        return Ok(Some(id));
    }
    report.created("Klanten");
    if dry_run {
        return Ok(Some(Uuid::nil()));
    }
    let id = Uuid::new_v4();
    let unusable = hex::encode(rand::random::<[u8; 32]>());
    sqlx::query(
        "INSERT INTO users (id, email, password_hash, first_name, last_name, role) VALUES ($1, $2, $3, $4, $5, 'customer')",
    )
    .bind(id)
    .bind(&email)
    .bind(auth::hash_password(&unusable)?)
    .bind(attributes["first_name"].as_str().unwrap_or_default())
    .bind(attributes["last_name"].as_str().unwrap_or_default())
    .execute(pool)
    .await?;
    Ok(Some(id))
}

pub async fn run(
    pool: &PgPool,
    integration_id: Uuid,
    client: &Pterodactyl,
    dry_run: bool,
) -> anyhow::Result<Report> {
    let mut report = Report::default();
    let users = client.users().await?;
    let servers: Vec<ImportedServer> = client
        .servers()
        .await?
        .iter()
        .filter_map(server_from)
        .collect();

    let owners: std::collections::HashSet<i64> = servers
        .iter()
        .map(|server| server.owner_external_id)
        .collect();
    let mut accounts: HashMap<i64, Uuid> = HashMap::new();
    for user in &users {
        let Some(external_id) = user["attributes"]["id"].as_i64() else {
            continue;
        };
        let is_admin = user["attributes"]["root_admin"].as_bool().unwrap_or(false);
        if is_admin && !owners.contains(&external_id) {
            report.skipped.push(format!(
                "Beheerder {} heeft geen servers en is overgeslagen",
                user["attributes"]["email"].as_str().unwrap_or("?")
            ));
            continue;
        }
        if let Some(id) = ensure_user(pool, user, dry_run, &mut report).await? {
            accounts.insert(external_id, id);
        }
    }

    for server in servers {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM services WHERE integration_id = $1 AND external_id = $2)",
        )
        .bind(integration_id)
        .bind(&server.external_id)
        .fetch_one(pool)
        .await?;
        if exists {
            report.existing("Diensten");
            continue;
        }
        let Some(user_id) = accounts.get(&server.owner_external_id).copied() else {
            report.skipped.push(format!(
                "Server {} heeft geen eigenaar met een e-mailadres",
                server.name
            ));
            continue;
        };
        report.created("Diensten");
        if dry_run {
            continue;
        }
        sqlx::query(
            "INSERT INTO services (id, user_id, integration_id, kind, name, status, external_id, details)
             VALUES ($1, $2, $3, 'gameserver', $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(integration_id)
        .bind(&server.name)
        .bind(if server.suspended { "suspended" } else { "active" })
        .bind(&server.external_id)
        .bind(&server.details)
        .execute(pool)
        .await?;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::server_from;

    #[test]
    fn reads_the_primary_allocation_and_limits() {
        let value = json!({
            "attributes": {
                "id": 7, "user": 3, "name": "Lobby", "suspended": true, "identifier": "ab12cd34", "uuid": "u",
                "allocation": 21,
                "limits": { "memory": 2048, "disk": 10240, "cpu": 150 },
                "relationships": {
                    "allocations": { "data": [
                        { "attributes": { "id": 20, "ip": "10.0.0.1", "alias": null, "port": 25565 } },
                        { "attributes": { "id": 21, "ip": "10.0.0.1", "alias": "play.voorbeeld.nl", "port": 25570 } }
                    ] },
                    "egg": { "attributes": { "name": "Paper" } }
                }
            }
        });
        let server = server_from(&value).unwrap();
        assert_eq!(server.external_id, "7");
        assert_eq!(server.owner_external_id, 3);
        assert!(server.suspended);
        assert_eq!(server.details["address"], "play.voorbeeld.nl:25570");
        assert_eq!(server.details["memory"], 2048);
        assert_eq!(server.details["egg"], "Paper");
    }
}
