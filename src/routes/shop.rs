use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::proxmox;
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
    id: u32,
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
        .zip(1..)
        .map(|(name, id)| ImageOption {
            id,
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
                            id: image.template,
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
    image: Option<u32>,
    #[serde(default)]
    ssh_key_ids: Vec<Uuid>,
}

#[derive(Serialize)]
pub struct OrderPlaced {
    invoice_id: i64,
    subscription_code: String,
    service_id: Uuid,
}

fn subscription_code() -> String {
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
        _ => "Dienst",
    }
}

pub async fn place(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<NewOrder>,
) -> AppResult<(StatusCode, Json<OrderPlaced>)> {
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
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Geef de server een naam van hoogstens 60 tekens.",
        ));
    }
    let images = images(&product);
    let image = match (input.image, images.first()) {
        (Some(wanted), _) if images.iter().any(|image| image.id == wanted) => Some(wanted),
        (Some(_), _) => return Err(AppError::field("image", "Kies een besturingssysteem.")),
        (None, Some(first)) if product.kind == "vps" => Some(first.id),
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
    let description = format!("{}: {}", kind_label(&product.kind), product.name);

    let mut transaction = state.db.begin().await?;
    sqlx::query(
        "INSERT INTO subscriptions (id, code, user_id, status, billing_interval, payment_mode, mandate_id, expires_at)
         SELECT $1, $2, u.id, 'pending', $3, u.default_payment_mode, u.default_mandate_id, $4 FROM users u WHERE u.id = $5",
    )
    .bind(subscription_id)
    .bind(&code)
    .bind(&input.billing_interval)
    .bind(today)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO subscription_items (id, subscription_id, product_id, description, monthly_price_cents)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(subscription_id)
    .bind(product.id)
    .bind(&description)
    .bind(product.monthly_price_cents)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO services (id, user_id, product_id, subscription_id, integration_id, kind, name, status, details)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending', $8)",
    )
    .bind(service_id)
    .bind(user.id)
    .bind(product.id)
    .bind(subscription_id)
    .bind(product.integration_id)
    .bind(&product.kind)
    .bind(name)
    .bind(json!({ "image": image, "ssh_key_ids": input.ssh_key_ids }))
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    let lines = [Line {
        subscription_id: Some(subscription_id),
        description,
        period_start: Some(today.date()),
        period_end: Some(period_end),
        amount_cents: i64::from(product.monthly_price_cents) * months,
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
