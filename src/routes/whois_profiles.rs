use std::collections::BTreeMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::integrations::{self, openprovider};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct WhoisProfile {
    pub id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub company: Option<String>,
    pub email: String,
    pub phone: String,
    pub street: String,
    pub house_number: String,
    pub zipcode: String,
    pub city: String,
    pub country: String,
    pub domains: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

impl WhoisProfile {
    pub fn contact(&self) -> openprovider::Contact<'_> {
        openprovider::Contact {
            first_name: &self.first_name,
            last_name: &self.last_name,
            company: self.company.as_deref(),
            email: &self.email,
            phone: &self.phone,
            street: &self.street,
            number: &self.house_number,
            zipcode: &self.zipcode,
            city: &self.city,
            country: &self.country,
        }
    }
}

#[derive(Deserialize)]
pub struct ProfileInput {
    first_name: String,
    last_name: String,
    #[serde(default)]
    company: Option<String>,
    email: String,
    phone: String,
    street: String,
    house_number: String,
    zipcode: String,
    city: String,
    country: String,
}

const SELECT: &str = "SELECT p.id, p.first_name, p.last_name, p.company, p.email, p.phone, p.street, p.house_number,
        p.zipcode, p.city, p.country, p.created_at,
        (SELECT count(*) FROM services s WHERE s.kind = 'domain' AND s.status NOT IN ('terminated', 'failed')
            AND s.details->>'whois_profile_id' = p.id::text) AS domains
     FROM whois_profiles p WHERE p.user_id = $1";

pub async fn load(state: &AppState, user_id: Uuid, id: Uuid) -> AppResult<WhoisProfile> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} AND p.id = $2")))
        .bind(user_id)
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)
}

fn normalize_phone(phone: &str) -> String {
    let phone = phone.trim();
    let phone = if phone.contains('.') {
        phone.to_owned()
    } else {
        phone.replacen(' ', ".", 1)
    };
    phone.chars().filter(|c| !c.is_whitespace()).collect()
}

fn clean(input: ProfileInput) -> Result<ProfileInput, AppError> {
    let mut errors = BTreeMap::new();
    let trim = |value: String| value.trim().to_owned();
    let input = ProfileInput {
        first_name: trim(input.first_name),
        last_name: trim(input.last_name),
        company: input
            .company
            .map(trim)
            .filter(|company| !company.is_empty()),
        email: trim(input.email).to_lowercase(),
        phone: normalize_phone(&input.phone),
        street: trim(input.street),
        house_number: trim(input.house_number),
        zipcode: trim(input.zipcode).to_uppercase(),
        city: trim(input.city),
        country: trim(input.country).to_uppercase(),
    };
    for (field, value, message) in [
        ("first_name", &input.first_name, "Vul een voornaam in."),
        ("last_name", &input.last_name, "Vul een achternaam in."),
        ("street", &input.street, "Vul een straat in."),
        (
            "house_number",
            &input.house_number,
            "Vul een huisnummer in.",
        ),
        ("zipcode", &input.zipcode, "Vul een postcode in."),
        ("city", &input.city, "Vul een plaats in."),
    ] {
        if value.is_empty() || value.chars().count() > 100 {
            errors.insert(field, message.to_owned());
        }
    }
    let at = input.email.find('@');
    if !at.is_some_and(|at| at > 0 && input.email[at + 1..].contains('.')) {
        errors.insert("email", "Vul een geldig e-mailadres in.".to_owned());
    }
    if openprovider::split_phone(&input.phone).is_none() {
        errors.insert(
            "phone",
            "Vul een telefoonnummer in met landcode, zoals +31 612345678.".to_owned(),
        );
    }
    if input.country.len() != 2 || !input.country.chars().all(|c| c.is_ascii_uppercase()) {
        errors.insert("country", "Kies een land.".to_owned());
    }
    if errors.is_empty() {
        Ok(input)
    } else {
        Err(AppError::Validation(errors))
    }
}

pub async fn index(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<Vec<WhoisProfile>>> {
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "{SELECT} ORDER BY p.created_at DESC"
        )))
        .bind(user.id)
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<ProfileInput>,
) -> AppResult<(StatusCode, Json<WhoisProfile>)> {
    let input = clean(input)?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO whois_profiles (id, user_id, first_name, last_name, company, email, phone, street, house_number, zipcode, city, country)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
    )
    .bind(id)
    .bind(user.id)
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(&input.company)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.street)
    .bind(&input.house_number)
    .bind(&input.zipcode)
    .bind(&input.city)
    .bind(&input.country)
    .execute(&state.db)
    .await?;
    Ok((StatusCode::CREATED, Json(load(&state, user.id, id).await?)))
}

pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<ProfileInput>,
) -> AppResult<Json<WhoisProfile>> {
    let current = load(&state, user.id, id).await?;
    let input = clean(input)?;
    if current.domains > 0
        && (input.first_name != current.first_name || input.last_name != current.last_name)
    {
        return Err(AppError::field(
            "first_name",
            "De naam van de houder wijzig je met een verhuizing naar een nieuw profiel.",
        ));
    }
    sqlx::query(
        "UPDATE whois_profiles SET first_name = $3, last_name = $4, company = $5, email = $6, phone = $7, street = $8,
            house_number = $9, zipcode = $10, city = $11, country = $12, updated_at = now()
         WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user.id)
    .bind(&input.first_name)
    .bind(&input.last_name)
    .bind(&input.company)
    .bind(&input.email)
    .bind(&input.phone)
    .bind(&input.street)
    .bind(&input.house_number)
    .bind(&input.zipcode)
    .bind(&input.city)
    .bind(&input.country)
    .execute(&state.db)
    .await?;
    let profile = load(&state, user.id, id).await?;

    let handles: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT integration_id, handle FROM whois_profile_handles WHERE whois_profile_id = $1",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await?;
    for (integration_id, handle) in handles {
        if let integrations::Driver::Openprovider(client) =
            integrations::driver(&state, integration_id).await?
        {
            client
                .update_handle(&handle, &profile.contact())
                .await
                .map_err(|error| AppError::upstream("profile", error))?;
        }
    }
    Ok(Json(profile))
}

pub async fn destroy(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    let profile = load(&state, user.id, id).await?;
    if profile.domains > 0 {
        return Err(AppError::field(
            "profile",
            "Dit profiel hoort nog bij een domein.",
        ));
    }
    sqlx::query("DELETE FROM whois_profiles WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(user.id)
        .execute(&state.db)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::normalize_phone;

    #[test]
    fn normalizes_phone_numbers() {
        assert_eq!(normalize_phone(" +31 6 1234 5678 "), "+31.612345678");
        assert_eq!(normalize_phone("+32.470123456"), "+32.470123456");
    }
}
