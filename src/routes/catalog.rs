use axum::Json;
use axum::extract::State;
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::error::AppResult;
use crate::limits::{self, ClientIp};
use crate::settings::{self, Billing, Company};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
struct ProductRow {
    id: Uuid,
    kind: String,
    name: String,
    description: Option<String>,
    specs: Value,
    monthly_price_cents: i32,
}

#[derive(Serialize)]
struct Product {
    id: Uuid,
    kind: String,
    name: String,
    description: Option<String>,
    specs: Value,
    monthly_price_cents: i32,
    order_url: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct Extension {
    extension: String,
    register_cents: i32,
    renew_cents: i32,
    transfer_cents: i32,
}

#[derive(Serialize)]
pub struct Catalog {
    company: Option<String>,
    currency: String,
    prices_include_vat: bool,
    products: Vec<Product>,
    domains: Vec<Extension>,
    domain_order_url: String,
}

fn order_path(kind: &str, id: Uuid) -> String {
    match kind {
        "vps" => format!("/vps/servers/order?product={id}"),
        "webhosting" => format!("/webhosting/order?product={id}"),
        _ => "/register".to_owned(),
    }
}

pub async fn index(State(state): State<AppState>, ClientIp(ip): ClientIp) -> AppResult<Response> {
    state.limits.check(&limits::CATALOG, &ip)?;
    let billing: Billing = settings::get(&state.db, "billing").await?;
    let company: Company = settings::get(&state.db, "company").await?;
    let rows: Vec<ProductRow> = sqlx::query_as(
        "SELECT id, kind, name, description, specs, monthly_price_cents FROM products
         WHERE is_active AND kind IN ('vps', 'webhosting', 'gameserver') AND (integration_id IS NOT NULL OR $1)
         ORDER BY kind, monthly_price_cents, name",
    )
    .bind(state.config.demo_mode)
    .fetch_all(&state.db)
    .await?;
    let domains = sqlx::query_as(
        "SELECT extension, register_cents, renew_cents, transfer_cents FROM domain_extensions
         WHERE is_active ORDER BY register_cents, extension",
    )
    .fetch_all(&state.db)
    .await?;
    let base = &state.config.public_url;
    let catalog = Catalog {
        company: (!company.name.trim().is_empty()).then_some(company.name),
        currency: billing.currency,
        prices_include_vat: true,
        products: rows
            .into_iter()
            .map(|row| Product {
                order_url: format!("{base}{}", order_path(&row.kind, row.id)),
                id: row.id,
                kind: row.kind,
                name: row.name,
                description: row.description,
                specs: row.specs,
                monthly_price_cents: row.monthly_price_cents,
            })
            .collect(),
        domains,
        domain_order_url: format!("{base}/domains/register"),
    };
    let mut response = Json(catalog).into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        HeaderValue::from_static("*"),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    Ok(response)
}
