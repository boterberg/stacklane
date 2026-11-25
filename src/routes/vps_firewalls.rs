use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use super::vps::{machine_of, validate_rule};
use crate::error::{AppError, AppResult};
use crate::integrations::vps::FirewallRule;
use crate::routes::vps_access::VpsActor;
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Firewall {
    id: Uuid,
    name: String,
    auto_assign: bool,
    rules: i64,
    servers: i64,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct AttachedServer {
    id: Uuid,
    name: String,
}

#[derive(Serialize)]
pub struct FirewallDetail {
    #[serde(flatten)]
    firewall: Firewall,
    rule_list: Vec<FirewallRule>,
    server_list: Vec<AttachedServer>,
}

#[derive(Deserialize)]
pub struct FirewallInput {
    name: String,
    #[serde(default)]
    auto_assign: bool,
    #[serde(default)]
    rules: Vec<FirewallRule>,
    #[serde(default)]
    server_ids: Option<Vec<Uuid>>,
}

#[derive(Serialize)]
pub struct SyncProblem {
    server: String,
    error: String,
}

#[derive(Serialize)]
pub struct Saved {
    id: Uuid,
    problems: Vec<SyncProblem>,
}

const SELECT: &str = "SELECT f.id, f.name, f.auto_assign, f.created_at,
        (SELECT count(*) FROM vps_firewall_rules r WHERE r.firewall_id = f.id) AS rules,
        (SELECT count(*) FROM vps_firewall_servers s JOIN services v ON v.id = s.service_id
            WHERE s.firewall_id = f.id AND v.status <> 'terminated') AS servers
     FROM vps_firewalls f WHERE f.user_id = $1";

pub fn marker(firewall_id: Uuid) -> String {
    let short: String = firewall_id.simple().to_string().chars().take(8).collect();
    format!("[fw:{short}]")
}

fn tagged(rule: &FirewallRule, marker: &str) -> FirewallRule {
    let comment = match rule.comment.as_deref().map(str::trim) {
        Some(text) if !text.is_empty() => format!("{marker} {text}"),
        _ => marker.to_owned(),
    };
    FirewallRule {
        id: String::new(),
        comment: Some(comment),
        ..rule.clone()
    }
}

pub fn split_ports(rule: &FirewallRule, lists_allowed: bool) -> Vec<FirewallRule> {
    match rule.dport.as_deref() {
        Some(ports) if !lists_allowed && ports.contains(',') => ports
            .split(',')
            .map(|port| FirewallRule {
                dport: Some(port.trim().to_owned()),
                ..rule.clone()
            })
            .collect(),
        _ => vec![rule.clone()],
    }
}

pub fn managed_ids(existing: &[FirewallRule], marker: &str) -> Vec<String> {
    let mut ids: Vec<String> = existing
        .iter()
        .filter(|rule| {
            rule.comment
                .as_deref()
                .is_some_and(|comment| comment.starts_with(marker))
        })
        .map(|rule| rule.id.clone())
        .collect();
    ids.sort_by_key(|id| std::cmp::Reverse(id.parse::<u64>().unwrap_or(0)));
    ids
}

async fn sync_server(
    state: &AppState,
    user_id: Uuid,
    service_id: Uuid,
    marker: &str,
    rules: &[FirewallRule],
) -> anyhow::Result<()> {
    let _guard = crate::locks::service(&state.db, service_id).await?;
    let machine = machine_of(state, user_id, service_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    if !machine.features().firewall {
        anyhow::bail!("dit platform heeft geen firewall");
    }
    if !machine.can_block() && rules.iter().any(|rule| rule.action != "ACCEPT") {
        anyhow::bail!("dit platform kan alleen verkeer toestaan, niet blokkeren");
    }
    let existing = machine.firewall_rules().await?;
    if !machine.can_block() {
        let kept = existing
            .iter()
            .filter(|rule| {
                !rule
                    .comment
                    .as_deref()
                    .is_some_and(|comment| comment.starts_with(marker))
            })
            .count();
        let added: usize = rules
            .iter()
            .map(|rule| split_ports(rule, false).len())
            .sum();
        if kept + added > SERVER_RULES {
            anyhow::bail!("deze server zou meer dan {SERVER_RULES} firewallregels krijgen");
        }
    }
    for id in managed_ids(&existing, marker) {
        machine.delete_firewall_rule(&id).await?;
    }
    for rule in rules.iter().rev() {
        for part in split_ports(rule, machine.can_block()) {
            machine.add_firewall_rule(&tagged(&part, marker)).await?;
        }
    }
    Ok(())
}

type RuleRow = (
    Uuid,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

async fn rules_of(state: &AppState, firewall_id: Uuid) -> AppResult<Vec<FirewallRule>> {
    let rows: Vec<RuleRow> = sqlx::query_as(
        "SELECT id, direction, action, proto, dport, source, comment FROM vps_firewall_rules
             WHERE firewall_id = $1 ORDER BY position",
    )
    .bind(firewall_id)
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, direction, action, proto, dport, source, comment)| FirewallRule {
                id: id.to_string(),
                direction,
                action,
                proto,
                dport,
                source,
                comment,
            },
        )
        .collect())
}

async fn server_name(state: &AppState, service_id: Uuid) -> String {
    sqlx::query_scalar("SELECT name FROM services WHERE id = $1")
        .bind(service_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| service_id.to_string())
}

async fn sync_all(
    state: &AppState,
    user_id: Uuid,
    firewall_id: Uuid,
    servers: &[Uuid],
    rules: &[FirewallRule],
) -> Vec<SyncProblem> {
    let marker = marker(firewall_id);
    let mut problems = Vec::new();
    for service_id in servers {
        if let Err(error) = sync_server(state, user_id, *service_id, &marker, rules).await {
            problems.push(SyncProblem {
                server: server_name(state, *service_id).await,
                error: {
                    tracing::warn!(error = %format!("{error:#}"), "syncing a firewall failed");
                    crate::i18n::tr(&crate::error::public_message(&error.to_string()))
                },
            });
        }
    }
    problems
}

async fn attached(state: &AppState, firewall_id: Uuid) -> AppResult<Vec<Uuid>> {
    Ok(sqlx::query_scalar(
        "SELECT s.service_id FROM vps_firewall_servers s JOIN services v ON v.id = s.service_id
         WHERE s.firewall_id = $1 AND v.status = 'active'",
    )
    .bind(firewall_id)
    .fetch_all(&state.db)
    .await?)
}

async fn owned(state: &AppState, actor: &VpsActor, id: Uuid) -> AppResult<Firewall> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND f.id = $2")))
        .bind(actor.owner)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)
}

fn clean(input: &mut FirewallInput) -> AppResult<()> {
    input.name = input.name.trim().to_owned();
    if input.name.is_empty() || input.name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Geef de firewall een naam van hoogstens 60 tekens.",
        ));
    }
    if input.rules.len() > 50 {
        return Err(AppError::field(
            "rules",
            "Een firewall heeft hoogstens 50 regels.",
        ));
    }
    for rule in &mut input.rules {
        validate_rule(rule)?;
    }
    let entries: usize = input
        .rules
        .iter()
        .map(|rule| {
            rule.dport
                .as_deref()
                .map_or(1, |ports| ports.split(',').count())
        })
        .sum();
    if entries > 100 {
        return Err(AppError::field(
            "rules",
            "Een firewall heeft samen hoogstens 100 poorten of reeksen.",
        ));
    }
    Ok(())
}

async fn eligible(state: &AppState, user_id: Uuid) -> AppResult<Vec<AttachedServer>> {
    let servers: Vec<AttachedServer> = sqlx::query_as(
        "SELECT id, name FROM services WHERE user_id = $1 AND kind = 'vps' AND status = 'active' ORDER BY name",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    let mut capable = Vec::new();
    for server in servers {
        if let Ok(machine) = machine_of(state, user_id, server.id).await
            && machine.features().firewall
        {
            capable.push(server);
        }
    }
    Ok(capable)
}

pub async fn servers(
    State(state): State<AppState>,
    actor: VpsActor,
) -> AppResult<Json<Vec<AttachedServer>>> {
    actor.require("firewalls.view")?;
    Ok(Json(eligible(&state, actor.owner).await?))
}

const MAX_FIREWALLS: i64 = 20;

async fn store(
    state: &AppState,
    actor: &VpsActor,
    id: Uuid,
    input: &FirewallInput,
) -> AppResult<Vec<Uuid>> {
    let wanted: Option<Vec<Uuid>> = match &input.server_ids {
        Some(ids) => {
            let capable = eligible(state, actor.owner).await?;
            if let Some(other) = ids
                .iter()
                .find(|id| !capable.iter().any(|server| server.id == **id))
            {
                return Err(AppError::field(
                    "server_ids",
                    if sqlx::query_scalar::<_, bool>(
                        "SELECT EXISTS (SELECT 1 FROM services WHERE id = $1 AND user_id = $2)",
                    )
                    .bind(other)
                    .bind(actor.owner)
                    .fetch_one(&state.db)
                    .await?
                    {
                        "Deze server kan geen firewall van Stacklane gebruiken."
                    } else {
                        "Kies een van je eigen servers."
                    },
                ));
            }
            Some(ids.clone())
        }
        None => None,
    };
    let mut transaction = state.db.begin().await?;
    crate::locks::transaction(&mut transaction, format!("firewalls {}", actor.owner)).await?;
    let others: i64 =
        sqlx::query_scalar("SELECT count(*) FROM vps_firewalls WHERE user_id = $1 AND id <> $2")
            .bind(actor.owner)
            .bind(id)
            .fetch_one(&mut *transaction)
            .await?;
    if others >= MAX_FIREWALLS {
        return Err(AppError::field(
            "name",
            "Je hebt al 20 firewalls. Verwijder er eerst een.",
        ));
    }
    sqlx::query(
        "INSERT INTO vps_firewalls (id, user_id, name, auto_assign) VALUES ($1, $2, $3, $4)
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, auto_assign = EXCLUDED.auto_assign, updated_at = now()",
    )
    .bind(id)
    .bind(actor.owner)
    .bind(&input.name)
    .bind(input.auto_assign)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("DELETE FROM vps_firewall_rules WHERE firewall_id = $1")
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    for (position, rule) in input.rules.iter().enumerate() {
        sqlx::query(
            "INSERT INTO vps_firewall_rules (id, firewall_id, position, direction, action, proto, dport, source, comment)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(i32::try_from(position).unwrap_or(i32::MAX))
        .bind(&rule.direction)
        .bind(&rule.action)
        .bind(rule.proto.as_deref().filter(|value| !value.is_empty()))
        .bind(rule.dport.as_deref().filter(|value| !value.is_empty()))
        .bind(rule.source.as_deref().filter(|value| !value.is_empty()))
        .bind(&rule.comment)
        .execute(&mut *transaction)
        .await?;
    }
    let mut detached = Vec::new();
    if let Some(wanted) = &wanted {
        detached = sqlx::query_scalar(
            "DELETE FROM vps_firewall_servers WHERE firewall_id = $1 AND NOT (service_id = ANY($2)) RETURNING service_id",
        )
        .bind(id)
        .bind(wanted)
        .fetch_all(&mut *transaction)
        .await?;
        for service_id in wanted {
            sqlx::query(
                "INSERT INTO vps_firewall_servers (firewall_id, service_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(service_id)
            .execute(&mut *transaction)
            .await?;
        }
    }
    let crowded: Option<Uuid> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT fs.service_id FROM vps_firewall_servers fs
         JOIN vps_firewall_rules r ON r.firewall_id = fs.firewall_id
         WHERE fs.service_id IN (SELECT service_id FROM vps_firewall_servers WHERE firewall_id = $1)
         GROUP BY fs.service_id HAVING sum({ENTRIES}) > $2 LIMIT 1"
    )))
    .bind(id)
    .bind(SERVER_ENTRIES)
    .fetch_optional(&mut *transaction)
    .await?;
    if crowded.is_some() {
        return Err(AppError::field(
            "server_ids",
            "Een server kan samen hoogstens 100 poorten of reeksen uit firewalls krijgen. Koppel minder firewalls aan deze server.",
        ));
    }
    transaction.commit().await?;
    Ok(detached)
}

const SERVER_ENTRIES: i64 = 100;
const SERVER_RULES: usize = 100;
const ENTRIES: &str = "coalesce(array_length(string_to_array(r.dport, ','), 1), 1)";

async fn entries(state: &AppState, filter: &str, id: Uuid) -> anyhow::Result<i64> {
    Ok(sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT coalesce(sum({ENTRIES}), 0)::bigint FROM vps_firewall_rules r {filter}"
    )))
    .bind(id)
    .fetch_one(&state.db)
    .await?)
}

fn rule_key(rule: &FirewallRule) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}",
        rule.direction,
        rule.action,
        rule.proto.as_deref().unwrap_or_default(),
        rule.dport.as_deref().unwrap_or_default(),
        rule.source.as_deref().unwrap_or_default(),
        rule.comment.as_deref().unwrap_or_default()
    )
}

fn changed(before: &[String], after: &[String]) -> (usize, usize) {
    let mut left: Vec<&String> = before.iter().collect();
    let mut added = 0;
    for item in after {
        if let Some(position) = left.iter().position(|old| *old == item) {
            left.remove(position);
        } else {
            added += 1;
        }
    }
    (added, left.len())
}

fn kept_order<'a>(from: &'a [String], other: &[String]) -> Vec<&'a String> {
    let mut pool: Vec<&String> = other.iter().collect();
    from.iter()
        .filter(
            |item| match pool.iter().position(|candidate| candidate == item) {
                Some(position) => {
                    pool.remove(position);
                    true
                }
                None => false,
            },
        )
        .collect()
}

async fn check_rights(
    state: &AppState,
    actor: &VpsActor,
    id: Option<Uuid>,
    input: &FirewallInput,
) -> AppResult<()> {
    if actor.is_owner() {
        return Ok(());
    }
    let (rules, servers, auto_assign) = match id {
        Some(id) => (
            rules_of(state, id).await?,
            sqlx::query_scalar(
                "SELECT s.service_id FROM vps_firewall_servers s JOIN services v ON v.id = s.service_id
                 WHERE s.firewall_id = $1 AND v.status <> 'terminated'",
            )
            .bind(id)
            .fetch_all(&state.db)
            .await?,
            owned(state, actor, id).await?.auto_assign,
        ),
        None => (Vec::new(), Vec::new(), false),
    };
    let before: Vec<String> = rules.iter().map(rule_key).collect();
    let after: Vec<String> = input.rules.iter().map(rule_key).collect();
    let (added, removed) = changed(&before, &after);
    let updated = added.min(removed);
    if updated > 0 || kept_order(&before, &after) != kept_order(&after, &before) {
        actor.require("firewalls.rule_update")?;
    }
    if input.auto_assign && !auto_assign {
        actor.require("firewalls.attach")?;
    }
    if !input.auto_assign && auto_assign {
        actor.require("firewalls.detach")?;
    }
    if added > updated {
        actor.require("firewalls.rule_create")?;
    }
    if removed > updated {
        actor.require("firewalls.rule_delete")?;
    }
    if let Some(wanted) = &input.server_ids {
        if wanted.iter().any(|server| !servers.contains(server)) {
            actor.require("firewalls.attach")?;
        }
        if servers.iter().any(|server| !wanted.contains(server)) {
            actor.require("firewalls.detach")?;
        }
    }
    Ok(())
}

pub async fn index(
    State(state): State<AppState>,
    actor: VpsActor,
) -> AppResult<Json<Vec<Firewall>>> {
    actor.require("firewalls.view")?;
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{SELECT} ORDER BY f.created_at"
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
) -> AppResult<Json<FirewallDetail>> {
    actor.require("firewalls.view")?;
    let firewall = owned(&state, &actor, id).await?;
    let server_list = sqlx::query_as(
        "SELECT v.id, v.name FROM vps_firewall_servers s JOIN services v ON v.id = s.service_id
         WHERE s.firewall_id = $1 AND v.status <> 'terminated' ORDER BY v.name",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(FirewallDetail {
        firewall,
        rule_list: rules_of(&state, id).await?,
        server_list,
    }))
}

pub async fn create(
    State(state): State<AppState>,
    actor: VpsActor,
    Json(mut input): Json<FirewallInput>,
) -> AppResult<(StatusCode, Json<Saved>)> {
    actor.require("firewalls.create")?;
    clean(&mut input)?;
    check_rights(&state, &actor, None, &input).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM vps_firewalls WHERE user_id = $1")
        .bind(actor.owner)
        .fetch_one(&state.db)
        .await?;
    if count >= MAX_FIREWALLS {
        return Err(AppError::field(
            "name",
            "Je hebt al 20 firewalls. Verwijder er eerst een.",
        ));
    }
    let id = Uuid::new_v4();
    store(&state, &actor, id, &input).await?;
    let problems = sync_all(
        &state,
        actor.owner,
        id,
        &attached(&state, id).await?,
        &input.rules,
    )
    .await;
    Ok((StatusCode::CREATED, Json(Saved { id, problems })))
}

pub async fn update(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(mut input): Json<FirewallInput>,
) -> AppResult<Json<Saved>> {
    actor.require("firewalls.update")?;
    owned(&state, &actor, id).await?;
    clean(&mut input)?;
    check_rights(&state, &actor, Some(id), &input).await?;
    let detached = store(&state, &actor, id, &input).await?;
    let mut problems = sync_all(&state, actor.owner, id, &detached, &[]).await;
    problems.extend(
        sync_all(
            &state,
            actor.owner,
            id,
            &attached(&state, id).await?,
            &input.rules,
        )
        .await,
    );
    Ok(Json(Saved { id, problems }))
}

pub async fn destroy(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Saved>> {
    actor.require("firewalls.delete")?;
    owned(&state, &actor, id).await?;
    let waiting: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM vps_firewall_servers s JOIN services v ON v.id = s.service_id
                        WHERE s.firewall_id = $1 AND v.status NOT IN ('active', 'terminated'))",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await?;
    if waiting {
        return Err(AppError::field(
            "firewall",
            "Een gekoppelde server is nu niet actief, dus de regels kunnen er niet af. Koppel die server eerst los of wacht tot hij weer actief is.",
        ));
    }
    let problems = sync_all(&state, actor.owner, id, &attached(&state, id).await?, &[]).await;
    if !problems.is_empty() {
        let servers: Vec<&str> = problems
            .iter()
            .map(|problem| problem.server.as_str())
            .collect();
        return Err(AppError::field(
            "firewall",
            format!(
                "De regels konden niet van {} worden gehaald. De firewall blijft staan; probeer het later opnieuw.",
                servers.join(", ")
            ),
        ));
    }
    sqlx::query("DELETE FROM vps_firewalls WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(actor.owner)
        .execute(&state.db)
        .await?;
    Ok(Json(Saved { id, problems }))
}

pub async fn apply_automatic(state: &AppState, service_id: Uuid) -> anyhow::Result<()> {
    let user_id: Uuid = sqlx::query_scalar("SELECT user_id FROM services WHERE id = $1")
        .bind(service_id)
        .fetch_one(&state.db)
        .await?;
    let firewalls: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM vps_firewalls WHERE user_id = $1 AND auto_assign ORDER BY created_at",
    )
    .bind(user_id)
    .fetch_all(&state.db)
    .await?;
    for firewall_id in firewalls {
        let used = entries(
            state,
            "JOIN vps_firewall_servers fs ON fs.firewall_id = r.firewall_id WHERE fs.service_id = $1",
            service_id,
        )
        .await?;
        let adds = entries(state, "WHERE r.firewall_id = $1", firewall_id).await?;
        if used + adds > SERVER_ENTRIES {
            tracing::warn!(%service_id, %firewall_id, "skipping an automatic firewall, the server has too many rules");
            continue;
        }
        sqlx::query(
            "INSERT INTO vps_firewall_servers (firewall_id, service_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        )
        .bind(firewall_id)
        .bind(service_id)
        .execute(&state.db)
        .await?;
        let rules = rules_of(state, firewall_id)
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        sync_server(state, user_id, service_id, &marker(firewall_id), &rules).await?;
    }
    Ok(())
}

#[cfg(test)]
mod rights_tests {
    use super::{changed, kept_order};

    #[test]
    fn notices_a_new_order_of_the_rules_that_stay() {
        let list = |items: &[&str]| {
            items
                .iter()
                .map(|item| (*item).to_owned())
                .collect::<Vec<_>>()
        };
        let same = |before: &[&str], after: &[&str]| {
            let (before, after) = (list(before), list(after));
            kept_order(&before, &after) == kept_order(&after, &before)
        };
        assert!(same(&["a", "b"], &["a", "b", "c"]));
        assert!(same(&["a", "b", "c"], &["a", "c"]));
        assert!(!same(&["a", "b"], &["b", "a"]));
        assert!(!same(&["a", "b"], &["c", "b", "a"]));
    }

    #[test]
    fn counts_added_and_removed_rules() {
        let list = |items: &[&str]| {
            items
                .iter()
                .map(|item| (*item).to_owned())
                .collect::<Vec<_>>()
        };
        assert_eq!(changed(&list(&["a", "b"]), &list(&["a", "b"])), (0, 0));
        assert_eq!(changed(&list(&["a"]), &list(&["a", "c"])), (1, 0));
        assert_eq!(changed(&list(&["a", "b"]), &list(&["a"])), (0, 1));
        assert_eq!(changed(&list(&["a", "b"]), &list(&["a", "x"])), (1, 1));
        assert_eq!(changed(&list(&["a", "a"]), &list(&["a"])), (0, 1));
    }
}

#[cfg(test)]
mod tests {
    use super::{managed_ids, marker, split_ports, tagged};
    use crate::integrations::vps::FirewallRule;
    use uuid::Uuid;

    fn rule(id: &str, comment: Option<&str>) -> FirewallRule {
        FirewallRule {
            id: id.to_owned(),
            direction: "in".to_owned(),
            action: "ACCEPT".to_owned(),
            proto: Some("tcp".to_owned()),
            dport: Some("22".to_owned()),
            source: None,
            comment: comment.map(str::to_owned),
        }
    }

    #[test]
    fn tags_and_finds_managed_rules() {
        let id = Uuid::parse_str("1234abcd-0000-0000-0000-000000000000").unwrap();
        assert_eq!(marker(id), "[fw:1234abcd]");
        assert_eq!(
            tagged(&rule("", Some(" ssh ")), "[fw:1234abcd]")
                .comment
                .as_deref(),
            Some("[fw:1234abcd] ssh")
        );
        assert_eq!(
            tagged(&rule("", None), "[fw:1234abcd]").comment.as_deref(),
            Some("[fw:1234abcd]")
        );
        let existing = [
            rule("0", Some("eigen regel")),
            rule("1", Some("[fw:1234abcd] ssh")),
            rule("4", Some("[fw:1234abcd]")),
            rule("2", Some("[fw:99999999] web")),
        ];
        assert_eq!(managed_ids(&existing, "[fw:1234abcd]"), ["4", "1"]);
        let mut web = rule("", None);
        web.dport = Some("80, 443".to_owned());
        let ports = |list: Vec<FirewallRule>| {
            list.into_iter()
                .map(|rule| rule.dport.unwrap_or_default())
                .collect::<Vec<_>>()
        };
        assert_eq!(ports(split_ports(&web, false)), ["80", "443"]);
        assert_eq!(ports(split_ports(&web, true)), ["80, 443"]);
    }
}
