use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use crate::auth::CurrentAdmin;
use crate::error::{AppError, AppResult};
use crate::settings::{self, Billing, Company};
use crate::state::AppState;

#[derive(Serialize)]
pub struct Settings {
    company: Company,
    billing: Billing,
}

#[derive(Deserialize)]
pub struct SettingsUpdate {
    company: Option<Company>,
    billing: Option<Billing>,
}

fn trimmed(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn load(state: &AppState) -> AppResult<Settings> {
    Ok(Settings {
        company: settings::get(&state.db, "company").await?,
        billing: settings::get(&state.db, "billing").await?,
    })
}

pub async fn show(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<Settings>> {
    Ok(Json(load(&state).await?))
}

pub async fn update(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
    Json(input): Json<SettingsUpdate>,
) -> AppResult<Json<Settings>> {
    if let Some(company) = input.company {
        let company = Company {
            name: company.name.trim().to_owned(),
            address: trimmed(company.address),
            email: trimmed(company.email),
            website: trimmed(company.website),
            coc_number: trimmed(company.coc_number),
            vat_number: trimmed(company.vat_number),
            iban: trimmed(company.iban).map(|iban| iban.to_uppercase()),
            bic: trimmed(company.bic).map(|bic| bic.to_uppercase()),
        };
        if company.name.is_empty() {
            return Err(AppError::field("name", "Vul de naam van je bedrijf in."));
        }
        settings::set(&state.db, "company", &company).await?;
    }
    if let Some(mut billing) = input.billing {
        billing.home_country = billing.home_country.trim().to_uppercase();
        if crate::vat::eu_rate(&billing.home_country).is_none() {
            billing.oss = false;
        }
        if billing.home_country.len() != 2 {
            return Err(AppError::field(
                "home_country",
                "Kies het land van je bedrijf.",
            ));
        }
        if !(0..=10_000).contains(&billing.tax_rate_basis_points) {
            return Err(AppError::field(
                "tax_rate_basis_points",
                "Kies een btw-tarief tussen 0 en 100 procent.",
            ));
        }
        if [
            billing.payment_term_days,
            billing.renewal_notice_days,
            billing.suspend_after_days,
        ]
        .iter()
        .any(|days| !(0..=365).contains(days))
        {
            return Err(AppError::field(
                "billing",
                "Gebruik een aantal dagen tussen 0 en 365.",
            ));
        }
        settings::set(&state.db, "billing", &billing).await?;
    }
    Ok(Json(load(&state).await?))
}
