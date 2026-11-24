use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, openprovider};
use crate::invoicing::{self, Line};
use crate::payments::{conceal, renewal_end};
use crate::settings::{self, Billing};
use crate::state::AppState;

use super::shop::subscription_code;

#[derive(sqlx::FromRow, Clone)]
struct ExtensionRow {
    integration_id: Uuid,
    extension: String,
    register_cents: i32,
    renew_cents: i32,
    transfer_cents: i32,
}

#[derive(Serialize)]
pub struct SearchResult {
    domain: String,
    extension: String,
    available: bool,
    premium: bool,
    register_cents: i32,
    renew_cents: i32,
    transfer_cents: i32,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    query: String,
}

const MAX_SUGGESTIONS: usize = 8;

async fn extensions(state: &AppState) -> AppResult<Vec<ExtensionRow>> {
    Ok(sqlx::query_as(
        "SELECT integration_id, extension, register_cents, renew_cents, transfer_cents
         FROM domain_extensions WHERE is_active ORDER BY register_cents, extension",
    )
    .fetch_all(&state.db)
    .await?)
}

fn wanted(query: &str, known: &[ExtensionRow]) -> Option<Vec<(String, ExtensionRow)>> {
    let query = query
        .trim()
        .trim_end_matches('/')
        .trim_end_matches('.')
        .to_lowercase();
    let query = query
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    if let Some((name, extension)) = openprovider::split_domain(query) {
        let first = known.iter().find(|row| row.extension == extension)?.clone();
        let mut list = vec![(name.clone(), first)];
        list.extend(
            known
                .iter()
                .filter(|row| row.extension != extension)
                .take(MAX_SUGGESTIONS - 1)
                .map(|row| (name.clone(), row.clone())),
        );
        return Some(list);
    }
    let name = query.to_owned();
    openprovider::split_domain(&format!("{name}.nl"))?;
    Some(
        known
            .iter()
            .take(MAX_SUGGESTIONS)
            .map(|row| (name.clone(), row.clone()))
            .collect(),
    )
}

pub async fn search(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(query): Query<SearchQuery>,
) -> AppResult<Json<Vec<SearchResult>>> {
    let known = extensions(&state).await?;
    let Some(list) = wanted(&query.query, &known) else {
        return Err(AppError::field(
            "query",
            "Vul een domeinnaam in, zoals jouwbedrijf of jouwbedrijf.nl.",
        ));
    };
    let mut by_integration: BTreeMap<Uuid, Vec<(String, String)>> = BTreeMap::new();
    for (name, row) in &list {
        by_integration
            .entry(row.integration_id)
            .or_default()
            .push((name.clone(), row.extension.clone()));
    }
    let mut availability = BTreeMap::new();
    for (integration_id, domains) in by_integration {
        let integrations::Driver::Openprovider(client) =
            integrations::driver(&state, integration_id).await?
        else {
            continue;
        };
        let results = client
            .check(&domains)
            .await
            .map_err(|error| AppError::field("query", format!("{error:#}")))?;
        for result in results {
            availability.insert(result.domain.clone(), result);
        }
    }
    Ok(Json(
        list.into_iter()
            .filter_map(|(name, row)| {
                let domain = format!("{name}.{}", row.extension);
                let found = availability.get(&domain)?;
                Some(SearchResult {
                    available: found.available,
                    premium: found.premium,
                    domain,
                    extension: row.extension,
                    register_cents: row.register_cents,
                    renew_cents: row.renew_cents,
                    transfer_cents: row.transfer_cents,
                })
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct NewDomain {
    domain: String,
    whois_profile_id: Uuid,
    #[serde(default)]
    transfer: bool,
    #[serde(default)]
    auth_code: Option<String>,
}

#[derive(Serialize)]
pub struct DomainOrdered {
    invoice_id: i64,
    service_id: Uuid,
}

pub async fn order(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<NewDomain>,
) -> AppResult<(StatusCode, Json<DomainOrdered>)> {
    if state.config.demo_mode {
        return Err(AppError::field(
            "domain",
            "In de demo kun je niets bestellen.",
        ));
    }
    let Some((name, extension)) = openprovider::split_domain(&input.domain) else {
        return Err(AppError::field("domain", "Vul een geldige domeinnaam in."));
    };
    let domain = format!("{name}.{extension}");
    let row = extensions(&state)
        .await?
        .into_iter()
        .find(|row| row.extension == extension)
        .ok_or_else(|| AppError::field("domain", "Deze extensie kun je niet bestellen."))?;
    super::whois_profiles::load(&state, user.id, input.whois_profile_id)
        .await
        .map_err(|_| AppError::field("whois_profile_id", "Kies een WHOIS-profiel."))?;
    let auth_code = input
        .auth_code
        .as_deref()
        .map(str::trim)
        .filter(|code| !code.is_empty());
    if input.transfer && auth_code.is_none() {
        return Err(AppError::field(
            "auth_code",
            "Vul de autorisatiecode van je huidige provider in.",
        ));
    }

    let integrations::Driver::Openprovider(client) =
        integrations::driver(&state, row.integration_id).await?
    else {
        return Err(AppError::field(
            "domain",
            "Deze extensie kun je niet bestellen.",
        ));
    };
    let check = client
        .check(&[(name.clone(), extension.clone())])
        .await
        .map_err(|error| AppError::field("domain", format!("{error:#}")))?;
    let available = check.first().is_some_and(|result| result.available);
    let premium = check.first().is_some_and(|result| result.premium);
    if premium {
        return Err(AppError::field(
            "domain",
            "Dit is een premium domein. Neem contact met ons op voor de prijs.",
        ));
    }
    if input.transfer == available {
        return Err(AppError::field(
            "domain",
            if input.transfer {
                "Dit domein is nog vrij. Registreer het in plaats van het te verhuizen."
            } else {
                "Dit domein is al bezet."
            },
        ));
    }

    let billing: Billing = settings::get(&state.db, "billing").await?;
    let now = OffsetDateTime::now_utc();
    let subscription_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let description = format!("Domein: {domain}");
    let first_price = if input.transfer {
        row.transfer_cents
    } else {
        row.register_cents
    };
    let mut details = json!({
        "whois_profile_id": input.whois_profile_id,
        "operation": if input.transfer { "transfer" } else { "register" },
    });
    if let Some(code) = auth_code {
        details["auth_code"] = Value::String(conceal(&state, code)?);
    }

    let mut transaction = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, payment_mode, mandate_id, expires_at)
         SELECT $1, $2, u.id, 'pending', 'yearly', u.default_payment_mode, u.default_mandate_id, $3 FROM users u WHERE u.id = $4",
    )
    .bind(subscription_id)
    .bind(subscription_code())
    .bind(now)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO subscription_items (id, subscription_id, description, monthly_price_cents, period_price_cents)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(subscription_id)
    .bind(&description)
    .bind((row.renew_cents + 6) / 12)
    .bind(row.renew_cents)
    .execute(&mut *transaction)
    .await?;
    let inserted = sqlx::query(
        "INSERT INTO services (id, user_id, subscription_id, integration_id, kind, name, status, details)
         VALUES ($1, $2, $3, $4, 'domain', $5, 'pending', $6)",
    )
    .bind(service_id)
    .bind(user.id)
    .bind(subscription_id)
    .bind(row.integration_id)
    .bind(&domain)
    .bind(&details)
    .execute(&mut *transaction)
    .await;
    if let Err(sqlx::Error::Database(error)) = &inserted
        && error.is_unique_violation()
    {
        return Err(AppError::field("domain", "Dit domein is al besteld."));
    }
    inserted?;
    transaction.commit().await?;

    let lines = [Line {
        subscription_id: Some(subscription_id),
        description,
        period_start: Some(now.date()),
        period_end: Some(renewal_end(now, "yearly")),
        amount_cents: i64::from(first_price),
    }];
    let due_at = now.date() + Duration::days(i64::from(billing.payment_term_days));
    let (invoice_id, _) = invoicing::issue(&state, user.id, &lines, due_at, &billing).await?;
    Ok((
        StatusCode::CREATED,
        Json(DomainOrdered {
            invoice_id,
            service_id,
        }),
    ))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct OwnedDomain {
    id: Uuid,
    name: String,
    status: String,
    details: Value,
    last_error: Option<String>,
    subscription_code: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
    cancelled: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct DomainDetail {
    #[serde(flatten)]
    domain: OwnedDomain,
    registry: Option<openprovider::Domain>,
}

const SELECT: &str = "SELECT s.id, s.name, s.status, s.details - 'auth_code' AS details, s.last_error,
        sub.code AS subscription_code, sub.expires_at, sub.cancelled_at IS NOT NULL AS cancelled, s.created_at
     FROM services s LEFT JOIN subscriptions sub ON sub.id = s.subscription_id
     WHERE s.kind = 'domain' AND s.status <> 'terminated' AND s.user_id = $1";

pub async fn index(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<Vec<OwnedDomain>>> {
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} ORDER BY s.name")))
            .bind(user.id)
            .fetch_all(&state.db)
            .await?,
    ))
}

async fn registrar(
    state: &AppState,
    user: &CurrentUser,
    id: Uuid,
) -> AppResult<(openprovider::Openprovider, String)> {
    let (integration_id, external_id, status): (Option<Uuid>, Option<String>, String) =
        sqlx::query_as(
            "SELECT integration_id, external_id, status FROM services WHERE id = $1 AND user_id = $2 AND kind = 'domain'",
        )
        .bind(id)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;
    let (Some(integration_id), Some(external_id), "active") =
        (integration_id, external_id, status.as_str())
    else {
        return Err(AppError::field("domain", "Dit domein is nog niet klaar."));
    };
    match integrations::driver(state, integration_id).await? {
        integrations::Driver::Openprovider(client) => Ok((client, external_id)),
        _ => Err(AppError::NotFound),
    }
}

fn failed(error: anyhow::Error) -> AppError {
    AppError::field("domain", format!("{error:#}"))
}

pub async fn show(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<DomainDetail>> {
    let domain: OwnedDomain =
        sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND s.id = $2")))
            .bind(user.id)
            .bind(id)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    let registry = match registrar(&state, &user, id).await {
        Ok((client, external_id)) => client.domain(&external_id).await.ok(),
        Err(_) => None,
    };
    Ok(Json(DomainDetail { domain, registry }))
}

#[derive(Deserialize)]
pub struct Nameservers {
    nameservers: Vec<String>,
}

pub async fn set_nameservers(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<Nameservers>,
) -> AppResult<StatusCode> {
    let names: Vec<String> = input
        .nameservers
        .iter()
        .map(|name| name.trim().trim_end_matches('.').to_lowercase())
        .filter(|name| !name.is_empty())
        .collect();
    if names.len() < 2
        || names.len() > 6
        || names
            .iter()
            .any(|name| openprovider::split_domain(name).is_none())
    {
        return Err(AppError::field(
            "nameservers",
            "Vul twee tot zes geldige nameservers in.",
        ));
    }
    let (client, external_id) = registrar(&state, &user, id).await?;
    client
        .set_nameservers(&external_id, &names)
        .await
        .map_err(failed)?;
    sqlx::query(
        "UPDATE services SET details = jsonb_set(details, '{nameservers}', $2), updated_at = now() WHERE id = $1",
    )
    .bind(id)
    .bind(json!(names))
    .execute(&state.db)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct AuthCode {
    auth_code: String,
}

pub async fn auth_code(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AuthCode>> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    let (client, external_id) = registrar(&state, &user, id).await?;
    Ok(Json(AuthCode {
        auth_code: client.auth_code(&external_id).await.map_err(failed)?,
    }))
}

#[derive(Deserialize)]
pub struct Lock {
    locked: bool,
}

pub async fn set_lock(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<Lock>,
) -> AppResult<StatusCode> {
    let (client, external_id) = registrar(&state, &user, id).await?;
    client
        .set_locked(&external_id, input.locked)
        .await
        .map_err(failed)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::{ExtensionRow, wanted};
    use uuid::Uuid;

    fn row(extension: &str) -> ExtensionRow {
        ExtensionRow {
            integration_id: Uuid::nil(),
            extension: extension.to_owned(),
            register_cents: 0,
            renew_cents: 0,
            transfer_cents: 0,
        }
    }

    #[test]
    fn builds_the_domains_to_check() {
        let known = [row("nl"), row("com"), row("eu")];
        let names = |list: Vec<(String, ExtensionRow)>| -> Vec<String> {
            list.into_iter()
                .map(|(name, row)| format!("{name}.{}", row.extension))
                .collect()
        };
        assert_eq!(
            names(wanted("Voorbeeld", &known).unwrap()),
            ["voorbeeld.nl", "voorbeeld.com", "voorbeeld.eu"]
        );
        assert_eq!(
            names(wanted("https://www.voorbeeld.com/", &known).unwrap()),
            ["voorbeeld.com", "voorbeeld.nl", "voorbeeld.eu"]
        );
        assert!(wanted("voorbeeld.xyz", &known).is_none());
        assert!(wanted("geen spaties", &known).is_none());
    }
}
