use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::{openstack, proxmox, virtfusion};
use crate::invoicing::{self, Line};
use crate::payments::interval_months;
use crate::settings::{self, Billing};
use crate::state::AppState;

const INTERVALS: [&str; 4] = ["monthly", "quarterly", "half_yearly", "yearly"];

#[derive(Deserialize)]
pub struct ShopQuery {
    kind: String,
}

#[derive(Serialize)]
pub struct ImageOption {
    id: String,
    name: String,
}

#[derive(Serialize)]
pub struct ShopProduct {
    id: Uuid,
    name: String,
    description: Option<String>,
    specs: Value,
    monthly_price_cents: i32,
    images: Vec<ImageOption>,
}

#[derive(sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    kind: String,
    name: String,
    description: Option<String>,
    specs: Value,
    monthly_price_cents: i32,
    integration_id: Option<Uuid>,
    integration_kind: Option<String>,
    integration_settings: Option<Value>,
}

const PRODUCT_SELECT: &str =
    "SELECT p.id, p.kind, p.name, p.description, p.specs, p.monthly_price_cents, p.integration_id,
        i.kind AS integration_kind, i.settings AS integration_settings
     FROM products p LEFT JOIN integrations i ON i.id = p.integration_id
     WHERE p.is_active";

fn demo_images() -> Vec<ImageOption> {
    ["Ubuntu 24.04", "Debian 12", "Rocky Linux 9"]
        .into_iter()
        .map(|name| ImageOption {
            id: name.to_lowercase().replace(' ', "-"),
            name: name.to_owned(),
        })
        .collect()
}

fn images(row: &ProductRow) -> Vec<ImageOption> {
    match (row.integration_kind.as_deref(), &row.integration_settings) {
        (Some("proxmox"), Some(settings)) => {
            serde_json::from_value::<proxmox::Settings>(settings.clone())
                .map(|settings| {
                    settings
                        .images
                        .into_iter()
                        .map(|image| ImageOption {
                            id: image.template.to_string(),
                            name: image.name,
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        (Some("virtfusion"), Some(settings)) => {
            serde_json::from_value::<virtfusion::Settings>(settings.clone())
                .map(|settings| {
                    settings
                        .images
                        .into_iter()
                        .map(|image| ImageOption {
                            id: image.template.to_string(),
                            name: image.name,
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        (Some("openstack"), Some(settings)) => {
            serde_json::from_value::<openstack::Settings>(settings.clone())
                .map(|settings| {
                    settings
                        .images
                        .into_iter()
                        .map(|image| ImageOption {
                            id: image.id,
                            name: image.name,
                        })
                        .collect()
                })
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

pub async fn products(
    State(state): State<AppState>,
    _user: CurrentUser,
    Query(query): Query<ShopQuery>,
) -> AppResult<Json<Vec<ShopProduct>>> {
    let rows: Vec<ProductRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{PRODUCT_SELECT} AND p.kind = $1 AND (p.integration_id IS NOT NULL OR $2) ORDER BY p.monthly_price_cents"
    )))
    .bind(&query.kind)
    .bind(state.config.demo_mode)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| ShopProduct {
                images: if state.config.demo_mode
                    && row.integration_id.is_none()
                    && row.kind == "vps"
                {
                    demo_images()
                } else {
                    images(&row)
                },
                id: row.id,
                name: row.name,
                description: row.description,
                specs: row.specs,
                monthly_price_cents: row.monthly_price_cents,
            })
            .collect(),
    ))
}

#[derive(Deserialize)]
pub struct NewOrder {
    product_id: Uuid,
    billing_interval: String,
    name: String,
    image: Option<String>,
    #[serde(default)]
    ssh_key_ids: Vec<Uuid>,
    #[serde(default)]
    server_id: Option<Uuid>,
    #[serde(default)]
    size_gb: Option<u32>,
}

pub const MAX_VOLUMES: i64 = 10;
pub const MAX_FLOATING_IPS: i64 = 10;
pub const VOLUME_SIZES: std::ops::RangeInclusive<u32> = 10..=1000;

#[derive(Serialize)]
pub struct OrderPlaced {
    invoice_id: i64,
    subscription_code: String,
    service_id: Uuid,
}

pub(super) fn subscription_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    rand::random::<[u8; 6]>()
        .iter()
        .map(|byte| char::from(ALPHABET[usize::from(*byte) % ALPHABET.len()]))
        .collect()
}

fn kind_label(kind: &str) -> &'static str {
    match kind {
        "vps" => "VPS",
        "gameserver" => "Gameserver",
        "webhosting" => "Webhosting",
        "volume" => "Volume",
        "floating_ip" => "Floating IP",
        _ => "Dienst",
    }
}

async fn volume_order(
    state: &AppState,
    user: &CurrentUser,
    product: &ProductRow,
    server_id: Option<Uuid>,
    size_gb: Option<u32>,
) -> AppResult<(Uuid, u32)> {
    let size_gb = size_gb
        .filter(|size| VOLUME_SIZES.contains(size))
        .ok_or_else(|| {
            AppError::field(
                "size_gb",
                "Kies een grootte van minimaal 10 GB en maximaal 1 TB.",
            )
        })?;
    let server_id = server_id.ok_or_else(|| AppError::field("server_id", "Kies een server."))?;
    let same_integration: Option<bool> = sqlx::query_scalar(
        "SELECT integration_id IS NOT DISTINCT FROM $3 FROM services
         WHERE id = $1 AND user_id = $2 AND kind = 'vps' AND status = 'active'",
    )
    .bind(server_id)
    .bind(user.id)
    .bind(product.integration_id)
    .fetch_optional(&state.db)
    .await?;
    match same_integration {
        None => {
            return Err(AppError::field(
                "server_id",
                "Kies een van je actieve servers.",
            ));
        }
        Some(false) => {
            return Err(AppError::field(
                "server_id",
                "Dit volume kan niet bij deze server. Kies een server op hetzelfde platform.",
            ));
        }
        Some(true) => {}
    }
    let owned: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM services WHERE user_id = $1 AND kind = 'volume' AND status <> 'terminated'",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    if owned >= MAX_VOLUMES {
        return Err(AppError::field(
            "product_id",
            "Je hebt al 10 volumes. Zeg er eerst een op.",
        ));
    }
    Ok((server_id, size_gb))
}

struct NewSubscription<'a> {
    id: Uuid,
    code: &'a str,
    user_id: Uuid,
    product: &'a ProductRow,
    interval: &'a str,
    starts: OffsetDateTime,
    description: &'a str,
    monthly_price_cents: i32,
    service_id: Uuid,
    name: &'a str,
    details: Value,
}

async fn open_subscription(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    new: &NewSubscription<'_>,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, payment_mode, mandate_id, expires_at)
         SELECT $1, $2, u.id, 'pending', $3, u.default_payment_mode, u.default_mandate_id, $4 FROM users u WHERE u.id = $5",
    )
    .bind(new.id)
    .bind(new.code)
    .bind(new.interval)
    .bind(new.starts)
    .bind(new.user_id)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO subscription_items (id, subscription_id, product_id, description, monthly_price_cents)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(new.id)
    .bind(new.product.id)
    .bind(new.description)
    .bind(new.monthly_price_cents)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "INSERT INTO services (id, user_id, product_id, subscription_id, integration_id, kind, name, status, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', $8)",
    )
    .bind(new.service_id)
    .bind(new.user_id)
    .bind(new.product.id)
    .bind(new.id)
    .bind(new.product.integration_id)
    .bind(&new.product.kind)
    .bind(new.name)
    .bind(&new.details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
pub struct FloatingIpOrder {
    product_id: Uuid,
    billing_interval: String,
    quantity: u32,
}

#[derive(Serialize)]
pub struct FloatingIpsPlaced {
    invoice_id: i64,
    service_ids: Vec<Uuid>,
}

pub async fn place_floating_ips(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<FloatingIpOrder>,
) -> AppResult<(StatusCode, Json<FloatingIpsPlaced>)> {
    let key = format!("orders {}", user.id);
    crate::locks::exclusive(key, order_floating_ips(&state, &user, input)).await
}

fn sells_floating_ips(product: &ProductRow) -> bool {
    match (
        product.integration_kind.as_deref(),
        &product.integration_settings,
    ) {
        (Some("proxmox"), _) => true,
        (Some("openstack"), Some(settings)) => {
            serde_json::from_value::<openstack::Settings>(settings.clone()).is_ok_and(|settings| {
                settings
                    .floating_network_id
                    .is_some_and(|id| !id.trim().is_empty())
            })
        }
        _ => false,
    }
}

#[derive(Serialize)]
pub struct FloatingIpProduct {
    product_id: Uuid,
    name: String,
    monthly_price_cents: i32,
}

pub(super) async fn floating_ip_products(state: &AppState) -> AppResult<Vec<FloatingIpProduct>> {
    let rows: Vec<ProductRow> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{PRODUCT_SELECT} AND p.kind = 'floating_ip' AND p.integration_id IS NOT NULL
         ORDER BY p.monthly_price_cents, p.name"
    )))
    .fetch_all(&state.db)
    .await?;
    Ok(rows
        .into_iter()
        .filter(sells_floating_ips)
        .map(|row| FloatingIpProduct {
            product_id: row.id,
            name: row.name,
            monthly_price_cents: row.monthly_price_cents,
        })
        .collect())
}

async fn order_floating_ips(
    state: &AppState,
    user: &CurrentUser,
    input: FloatingIpOrder,
) -> AppResult<(StatusCode, Json<FloatingIpsPlaced>)> {
    if state.config.demo_mode {
        return Err(AppError::field(
            "product_id",
            "In de demo kun je niets bestellen.",
        ));
    }
    let product: ProductRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{PRODUCT_SELECT} AND p.id = $1 AND p.kind = 'floating_ip' AND p.integration_id IS NOT NULL"
    )))
    .bind(input.product_id)
    .fetch_optional(&state.db)
    .await?
    .filter(sells_floating_ips)
    .ok_or_else(|| AppError::field("product_id", "Kies waar je de floating IP wilt hebben."))?;
    if !INTERVALS.contains(&input.billing_interval.as_str()) {
        return Err(AppError::field(
            "billing_interval",
            "Kies hoe vaak je wilt betalen.",
        ));
    }
    let owned: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM services WHERE user_id = $1 AND kind = 'floating_ip' AND status <> 'terminated'",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;
    let left = u32::try_from((MAX_FLOATING_IPS - owned).max(0)).unwrap_or(0);
    if left == 0 {
        return Err(AppError::field(
            "quantity",
            "Je hebt al 10 floating IPs. Zeg er eerst een op.",
        ));
    }
    if input.quantity == 0 || input.quantity > left {
        return Err(AppError::field(
            "quantity",
            "Kies een aantal. Je kunt in totaal 10 floating IPs hebben.",
        ));
    }

    let billing: Billing = settings::get(&state.db, "billing").await?;
    let months = i64::from(interval_months(&input.billing_interval));
    let today = OffsetDateTime::now_utc();
    let period_end = crate::payments::renewal_end(today, &input.billing_interval);
    let description = "Floating IP".to_owned();
    let mut lines = Vec::new();
    let mut service_ids = Vec::new();
    let mut transaction = state.db.begin().await?;
    for _ in 0..input.quantity {
        let subscription_id = Uuid::new_v4();
        let service_id = Uuid::new_v4();
        open_subscription(
            &mut transaction,
            &NewSubscription {
                id: subscription_id,
                code: &subscription_code(),
                user_id: user.id,
                product: &product,
                interval: &input.billing_interval,
                starts: today,
                description: &description,
                monthly_price_cents: product.monthly_price_cents,
                service_id,
                name: "Floating IP",
                details: json!({}),
            },
        )
        .await?;
        lines.push(Line {
            subscription_id: Some(subscription_id),
            description: description.clone(),
            period_start: Some(today.date()),
            period_end: Some(period_end),
            amount_cents: i64::from(product.monthly_price_cents) * months,
        });
        service_ids.push(service_id);
    }
    transaction.commit().await?;

    let due_at = today.date() + Duration::days(i64::from(billing.payment_term_days));
    let (invoice_id, _) = invoicing::issue(state, user.id, &lines, due_at, &billing).await?;
    Ok((
        StatusCode::CREATED,
        Json(FloatingIpsPlaced {
            invoice_id,
            service_ids,
        }),
    ))
}

pub async fn place(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<NewOrder>,
) -> AppResult<(StatusCode, Json<OrderPlaced>)> {
    let key = format!("orders {}", user.id);
    crate::locks::exclusive(key, place_order(&state, user, input)).await
}

async fn place_order(
    state: &AppState,
    user: CurrentUser,
    input: NewOrder,
) -> AppResult<(StatusCode, Json<OrderPlaced>)> {
    let state = state.clone();
    if state.config.demo_mode {
        return Err(AppError::field(
            "product_id",
            "In de demo kun je niets bestellen.",
        ));
    }
    let product: ProductRow = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{PRODUCT_SELECT} AND p.id = $1 AND p.integration_id IS NOT NULL"
    )))
    .bind(input.product_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| AppError::field("product_id", "Kies een pakket."))?;
    if !INTERVALS.contains(&input.billing_interval.as_str()) {
        return Err(AppError::field(
            "billing_interval",
            "Kies hoe vaak je wilt betalen.",
        ));
    }
    let domain = input
        .name
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.")
        .trim_end_matches('/')
        .to_lowercase();
    let name = if product.kind == "webhosting" {
        domain.as_str()
    } else {
        input.name.trim()
    };
    if product.kind == "webhosting" && !crate::integrations::webhosting::valid_domain(name) {
        return Err(AppError::field(
            "name",
            "Vul de domeinnaam van je website in, zoals jouwbedrijf.nl.",
        ));
    }
    if product.kind == "webhosting" {
        let taken: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM services
                 WHERE lower(name) = $1
                   AND ((kind = 'webhosting' AND integration_id = $2 AND status IN ('provisioning', 'active', 'suspended'))
                        OR (kind = 'domain' AND user_id <> $3 AND status NOT IN ('terminated', 'failed'))))",
        )
        .bind(name)
        .bind(product.integration_id)
        .bind(user.id)
        .fetch_one(&state.db)
        .await?;
        if taken {
            return Err(AppError::field(
                "name",
                "Deze domeinnaam is al in gebruik bij een ander pakket.",
            ));
        }
    }
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Geef de server een naam van hoogstens 60 tekens.",
        ));
    }
    if product.kind == "floating_ip" {
        return Err(AppError::field(
            "product_id",
            "Bestel floating IPs onder VPS, Floating IPs.",
        ));
    }
    let volume = if product.kind == "volume" {
        Some(volume_order(&state, &user, &product, input.server_id, input.size_gb).await?)
    } else {
        None
    };
    let images = images(&product);
    let image = match (input.image, images.first()) {
        (Some(wanted), _) if images.iter().any(|image| image.id == wanted) => Some(wanted),
        (Some(_), _) => return Err(AppError::field("image", "Kies een besturingssysteem.")),
        (None, Some(first)) if product.kind == "vps" => Some(first.id.clone()),
        _ => None,
    };
    let owned: i64 =
        sqlx::query_scalar("SELECT count(*) FROM ssh_keys WHERE user_id = $1 AND id = ANY($2)")
            .bind(user.id)
            .bind(&input.ssh_key_ids)
            .fetch_one(&state.db)
            .await?;
    if usize::try_from(owned).unwrap_or(0) != input.ssh_key_ids.len() {
        return Err(AppError::field(
            "ssh_key_ids",
            "Kies je eigen SSH-sleutels.",
        ));
    }

    let billing: Billing = settings::get(&state.db, "billing").await?;
    let months = i64::from(interval_months(&input.billing_interval));
    let today = OffsetDateTime::now_utc();
    let period_end = crate::payments::renewal_end(today, &input.billing_interval);
    let code = subscription_code();
    let subscription_id = Uuid::new_v4();
    let service_id = Uuid::new_v4();
    let (description, monthly_price_cents, details) = match volume {
        Some((server_id, size_gb)) => (
            format!("Volume: {name} ({size_gb} GB)"),
            product
                .monthly_price_cents
                .saturating_mul(i32::try_from(size_gb).unwrap_or(i32::MAX)),
            json!({ "size_gb": size_gb, "server_id": server_id }),
        ),
        None => (
            format!("{}: {}", kind_label(&product.kind), product.name),
            product.monthly_price_cents,
            json!({ "image": image, "ssh_key_ids": input.ssh_key_ids }),
        ),
    };

    let mut transaction = state.db.begin().await?;
    open_subscription(
        &mut transaction,
        &NewSubscription {
            id: subscription_id,
            code: &code,
            user_id: user.id,
            product: &product,
            interval: &input.billing_interval,
            starts: today,
            description: &description,
            monthly_price_cents,
            service_id,
            name,
            details,
        },
    )
    .await?;
    transaction.commit().await?;

    let lines = [Line {
        subscription_id: Some(subscription_id),
        description,
        period_start: Some(today.date()),
        period_end: Some(period_end),
        amount_cents: i64::from(monthly_price_cents) * months,
    }];
    let due_at = today.date() + Duration::days(i64::from(billing.payment_term_days));
    let (invoice_id, _) = invoicing::issue(&state, user.id, &lines, due_at, &billing).await?;
    Ok((
        StatusCode::CREATED,
        Json(OrderPlaced {
            invoice_id,
            subscription_code: code,
            service_id,
        }),
    ))
}
