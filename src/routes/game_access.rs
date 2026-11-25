use std::collections::BTreeSet;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const PERMISSIONS: [&str; 5] = ["console", "power", "files", "backups", "activity"];
const MAX_MEMBERS: i64 = 10;
const NAME: &str = "coalesce(nullif(trim(u.company_name), ''), nullif(trim(u.first_name || ' ' || u.last_name), ''), u.email::text)";

pub struct Grant {
    pub owner: bool,
    pub permissions: Vec<String>,
}

impl Grant {
    pub fn can(&self, permission: &str) -> bool {
        self.owner || self.permissions.iter().any(|granted| granted == permission)
    }

    pub fn require(&self, permission: &str) -> AppResult<()> {
        if self.can(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

pub async fn grant(state: &AppState, user: Uuid, server: Uuid) -> AppResult<Grant> {
    let found: Option<(bool, Vec<String>)> = sqlx::query_as(
        "SELECT s.user_id = $2,
                coalesce((SELECT m.permissions FROM gameserver_members m
                          WHERE m.service_id = s.id AND m.user_id = $2 AND m.accepted_at IS NOT NULL), '{}')
         FROM services s
         WHERE s.id = $1 AND s.kind = 'gameserver' AND s.status <> 'terminated'
           AND (s.user_id = $2 OR EXISTS (
               SELECT 1 FROM gameserver_members m
               WHERE m.service_id = s.id AND m.user_id = $2 AND m.accepted_at IS NOT NULL))",
    )
    .bind(server)
    .bind(user)
    .fetch_optional(&state.db)
    .await?;
    let (owner, permissions) = found.ok_or(AppError::NotFound)?;
    Ok(Grant { owner, permissions })
}

async fn owned(state: &AppState, user: &CurrentUser, server: Uuid) -> AppResult<()> {
    if !grant(state, user.id, server).await?.owner {
        return Err(AppError::NotFound);
    }
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Member {
    id: Uuid,
    email: String,
    name: String,
    accepted: bool,
    permissions: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Shared {
    service_id: Uuid,
    server: String,
    owner: String,
    accepted: bool,
    permissions: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct NewMember {
    email: String,
    permissions: Vec<String>,
}

#[derive(Deserialize)]
pub struct Rights {
    permissions: Vec<String>,
}

fn clean(permissions: Vec<String>) -> AppResult<Vec<String>> {
    let set: BTreeSet<String> = permissions
        .into_iter()
        .map(|permission| permission.trim().to_owned())
        .collect();
    if set.is_empty() {
        return Err(AppError::field("permissions", "Kies minimaal één recht."));
    }
    if set
        .iter()
        .any(|permission| !PERMISSIONS.contains(&permission.as_str()))
    {
        return Err(AppError::field(
            "permissions",
            "Kies alleen rechten uit de lijst.",
        ));
    }
    Ok(set.into_iter().collect())
}

pub async fn index(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(server): Path<Uuid>,
) -> AppResult<Json<Vec<Member>>> {
    owned(&state, &user, server).await?;
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT m.id, u.email::text AS email, {NAME} AS name,
                    m.accepted_at IS NOT NULL AS accepted, m.permissions, m.created_at
             FROM gameserver_members m JOIN users u ON u.id = m.user_id
             WHERE m.service_id = $1 ORDER BY m.created_at"
        )))
        .bind(server)
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(server): Path<Uuid>,
    Json(input): Json<NewMember>,
) -> AppResult<StatusCode> {
    owned(&state, &user, server).await?;
    state
        .limits
        .check(&crate::limits::INVITE, &user.id.to_string())?;
    let permissions = clean(input.permissions)?;
    let email = input.email.trim();
    if email.is_empty() {
        return Err(AppError::field("email", "Vul een e-mailadres in."));
    }
    let owner_verified: bool =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id = $1")
            .bind(user.id)
            .fetch_one(&state.db)
            .await?;
    if !owner_verified {
        return Err(AppError::field(
            "email",
            "Bevestig eerst je eigen e-mailadres voordat je iemand toegang geeft.",
        ));
    }
    let invited: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, email_verified_at IS NOT NULL FROM users WHERE email = $1::citext AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?;
    let Some((invited, verified)) = invited else {
        return Err(AppError::field(
            "email",
            "Er is geen account met dit e-mailadres. Vraag de persoon eerst een account aan te maken.",
        ));
    };
    if !verified {
        return Err(AppError::field(
            "email",
            "Deze persoon heeft het e-mailadres nog niet bevestigd. Vraag dat eerst te doen.",
        ));
    }
    if invited == user.id {
        return Err(AppError::field("email", "Dit is je eigen account."));
    }
    let mut transaction = state.db.begin().await?;
    crate::locks::transaction(&mut transaction, format!("game members {server}")).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM gameserver_members WHERE service_id = $1")
            .bind(server)
            .fetch_one(&mut *transaction)
            .await?;
    if count >= MAX_MEMBERS {
        return Err(AppError::field(
            "email",
            "Je kunt hoogstens 10 gebruikers toegang geven.",
        ));
    }
    let inserted = sqlx::query(
        "INSERT INTO gameserver_members (id, service_id, user_id, permissions) VALUES ($1, $2, $3, $4)
         ON CONFLICT (service_id, user_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(server)
    .bind(invited)
    .bind(&permissions)
    .execute(&mut *transaction)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(AppError::field("email", "Deze gebruiker heeft al toegang."));
    }
    transaction.commit().await?;
    let (owner, name): (String, String) = sqlx::query_as(
        "SELECT u.email::text, s.name FROM users u JOIN services s ON s.id = $2 WHERE u.id = $1",
    )
    .bind(user.id)
    .bind(server)
    .fetch_one(&state.db)
    .await?;
    let _ = crate::renewals::notify(
        &state,
        invited,
        (
            "Uitnodiging voor een gameserver".to_owned(),
            format!(
                "Het account {owner} nodigt je uit voor de gameserver {name}. Onder Gedeelde toegang kun je de uitnodiging accepteren of weigeren."
            ),
        ),
        (
            "Invitation to a game server".to_owned(),
            format!(
                "The account {owner} invited you to the game server {name}. Accept or decline it under Shared access."
            ),
        ),
        "/account/shared-access",
    )
    .await;
    Ok(StatusCode::CREATED)
}

pub async fn update(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((server, member)): Path<(Uuid, Uuid)>,
    Json(input): Json<Rights>,
) -> AppResult<StatusCode> {
    owned(&state, &user, server).await?;
    let permissions = clean(input.permissions)?;
    let updated = sqlx::query(
        "UPDATE gameserver_members SET permissions = $3, updated_at = now() WHERE id = $1 AND service_id = $2",
    )
    .bind(member)
    .bind(server)
    .bind(&permissions)
    .execute(&state.db)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn destroy(
    State(state): State<AppState>,
    user: CurrentUser,
    Path((server, member)): Path<(Uuid, Uuid)>,
) -> AppResult<StatusCode> {
    owned(&state, &user, server).await?;
    let deleted = sqlx::query("DELETE FROM gameserver_members WHERE id = $1 AND service_id = $2")
        .bind(member)
        .bind(server)
        .execute(&state.db)
        .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn shared(
    State(state): State<AppState>,
    user: CurrentUser,
) -> AppResult<Json<Vec<Shared>>> {
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT s.id AS service_id, s.name AS server, {NAME} AS owner,
                    m.accepted_at IS NOT NULL AS accepted, m.permissions, m.created_at
             FROM gameserver_members m
             JOIN services s ON s.id = m.service_id
             JOIN users u ON u.id = s.user_id
             WHERE m.user_id = $1 AND s.status <> 'terminated' AND u.deleted_at IS NULL
             ORDER BY s.name"
        )))
        .bind(user.id)
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn accept(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(server): Path<Uuid>,
) -> AppResult<StatusCode> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    let accepted = sqlx::query(
        "UPDATE gameserver_members m SET accepted_at = now(), updated_at = now()
         FROM users u
         WHERE m.service_id = $1 AND m.user_id = $2 AND m.accepted_at IS NULL
           AND u.id = m.user_id AND u.email_verified_at IS NOT NULL",
    )
    .bind(server)
    .bind(user.id)
    .execute(&state.db)
    .await?;
    if accepted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn leave(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(server): Path<Uuid>,
) -> AppResult<StatusCode> {
    let deleted =
        sqlx::query("DELETE FROM gameserver_members WHERE service_id = $1 AND user_id = $2")
            .bind(server)
            .bind(user.id)
            .execute(&state.db)
            .await?;
    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn rights_come_from_the_list() {
        assert_eq!(
            clean(vec!["power".into(), " console ".into(), "power".into()]).unwrap(),
            vec!["console".to_owned(), "power".to_owned()]
        );
        assert!(clean(Vec::new()).is_err());
        assert!(clean(vec!["members".into()]).is_err());
    }
}
