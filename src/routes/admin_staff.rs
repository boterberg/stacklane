use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::{CurrentAdmin, PERMISSIONS};
use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Serialize, sqlx::FromRow)]
pub struct Member {
    id: Uuid,
    email: String,
    first_name: String,
    last_name: String,
    role: String,
    permissions: Vec<String>,
    two_factor: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    last_active_at: Option<OffsetDateTime>,
}

#[derive(Deserialize)]
pub struct Grant {
    #[serde(default)]
    email: String,
    role: String,
    #[serde(default)]
    permissions: Vec<String>,
}

const SELECT: &str = "SELECT u.id, u.email::text AS email, u.first_name, u.last_name, u.role,
        CASE u.role WHEN 'admin' THEN '{}'::text[] ELSE u.staff_permissions END AS permissions,
        u.totp_secret IS NOT NULL AS two_factor,
        (SELECT max(s.last_seen_at) FROM sessions s WHERE s.user_id = u.id) AS last_active_at
     FROM users u";

fn clean(role: &str, permissions: Vec<String>) -> AppResult<Vec<String>> {
    match role {
        "admin" => Ok(Vec::new()),
        "staff" => {
            let mut granted: Vec<String> = PERMISSIONS
                .iter()
                .filter(|key| permissions.iter().any(|wanted| wanted == *key))
                .map(|key| (*key).to_owned())
                .collect();
            granted.dedup();
            if permissions
                .iter()
                .any(|wanted| !PERMISSIONS.contains(&wanted.as_str()))
            {
                return Err(AppError::field(
                    "permissions",
                    "Kies alleen rechten uit de lijst.",
                ));
            }
            if granted.is_empty() {
                return Err(AppError::field(
                    "permissions",
                    "Geef een medewerker minstens één recht.",
                ));
            }
            Ok(granted)
        }
        _ => Err(AppError::field("role", "Kies medewerker of beheerder.")),
    }
}

async fn member(state: &AppState, id: Uuid) -> AppResult<Member> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!("{SELECT} WHERE u.id = $1")))
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)
}

pub async fn index(
    State(state): State<AppState>,
    _admin: CurrentAdmin,
) -> AppResult<Json<Vec<Member>>> {
    let members = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "{SELECT} WHERE u.role IN ('staff', 'admin') AND u.deleted_at IS NULL
         ORDER BY u.role = 'admin' DESC, lower(u.first_name), lower(u.last_name), u.email"
    )))
    .fetch_all(&state.db)
    .await?;
    Ok(Json(members))
}

pub async fn add(
    State(state): State<AppState>,
    admin: CurrentAdmin,
    Json(input): Json<Grant>,
) -> AppResult<Json<Member>> {
    admin.browser_only()?;
    let permissions = clean(&input.role, input.permissions)?;
    let email = input.email.trim();
    if email.is_empty() {
        return Err(AppError::field("email", "Vul een e-mailadres in."));
    }
    let found: Option<(Uuid, String, bool)> = sqlx::query_as(
        "SELECT id, role, email_verified_at IS NOT NULL FROM users WHERE email = $1::citext AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?;
    let (id, role, verified) = found.ok_or_else(|| {
        AppError::field(
            "email",
            "Er is geen account met dit e-mailadres. Laat diegene eerst zelf een account aanmaken.",
        )
    })?;
    if role != "customer" {
        return Err(AppError::field("email", "Deze persoon zit al in je team."));
    }
    if !verified {
        return Err(AppError::field(
            "email",
            "Deze persoon heeft het e-mailadres nog niet bevestigd. Vraag dat eerst te doen.",
        ));
    }
    let updated = sqlx::query(
        "UPDATE users SET role = $2, staff_permissions = $3
         WHERE id = $1 AND role = 'customer' AND email_verified_at IS NOT NULL AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(&input.role)
    .bind(&permissions)
    .execute(&state.db)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(member(&state, id).await?))
}

pub async fn update(
    State(state): State<AppState>,
    admin: CurrentAdmin,
    Path(id): Path<Uuid>,
    Json(input): Json<Grant>,
) -> AppResult<Json<Member>> {
    admin.browser_only()?;
    if id == admin.id {
        return Err(AppError::field(
            "role",
            "Je kunt je eigen rechten niet wijzigen.",
        ));
    }
    let permissions = clean(&input.role, input.permissions)?;
    let updated = sqlx::query(
        "UPDATE users SET role = $2, staff_permissions = $3
         WHERE id = $1 AND role IN ('staff', 'admin') AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(&input.role)
    .bind(&permissions)
    .execute(&state.db)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(member(&state, id).await?))
}

pub async fn remove(
    State(state): State<AppState>,
    admin: CurrentAdmin,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Member>> {
    admin.browser_only()?;
    if id == admin.id {
        return Err(AppError::field(
            "role",
            "Je kunt jezelf niet uit het team halen.",
        ));
    }
    let updated = sqlx::query(
        "UPDATE users SET role = 'customer', staff_permissions = '{}'
         WHERE id = $1 AND role IN ('staff', 'admin') AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(&state.db)
    .await?
    .rows_affected();
    if updated == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query("DELETE FROM api_tokens WHERE user_id = $1 AND EXISTS (SELECT 1 FROM unnest(scopes) scope WHERE scope LIKE 'admin:%')")
        .bind(id)
        .execute(&state.db)
        .await?;
    Ok(Json(member(&state, id).await?))
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn staff_needs_known_permissions() {
        assert_eq!(
            clean(
                "staff",
                vec!["billing".into(), "support".into(), "billing".into()]
            )
            .unwrap(),
            vec!["support".to_owned(), "billing".to_owned()]
        );
        assert!(clean("staff", Vec::new()).is_err());
        assert!(clean("staff", vec!["settings".into()]).is_err());
        assert!(clean("owner", vec!["support".into()]).is_err());
        assert!(clean("admin", vec!["support".into()]).unwrap().is_empty());
    }
}
