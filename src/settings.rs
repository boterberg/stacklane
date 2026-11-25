use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Company {
    pub name: String,
    pub address: Option<String>,
    pub email: Option<String>,
    pub website: Option<String>,
    pub coc_number: Option<String>,
    pub vat_number: Option<String>,
    pub iban: Option<String>,
    pub bic: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Billing {
    pub tax_rate_basis_points: i32,
    pub home_country: String,
    pub oss: bool,
    pub payment_term_days: i32,
    pub renewal_notice_days: i32,
    pub suspend_after_days: i32,
}

impl Default for Billing {
    fn default() -> Self {
        Self {
            tax_rate_basis_points: 2100,
            home_country: "NL".to_owned(),
            oss: false,
            payment_term_days: 14,
            renewal_notice_days: 14,
            suspend_after_days: 7,
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Support {
    pub auto_close_days: i32,
}

pub async fn get<T: DeserializeOwned + Default>(db: &PgPool, key: &str) -> sqlx::Result<T> {
    let value: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT value FROM settings WHERE key = $1")
            .bind(key)
            .fetch_optional(db)
            .await?;
    Ok(value
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default())
}

pub async fn set<T: Serialize>(db: &PgPool, key: &str, value: &T) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO settings (key, value) VALUES ($1, $2)
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(key)
    .bind(serde_json::to_value(value).unwrap_or_default())
    .execute(db)
    .await?;
    Ok(())
}
