use std::collections::BTreeSet;

use axum::Json;
use axum::extract::{FromRequestParts, Path, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::state::AppState;

pub const OWNER_HEADER: &str = "x-vps-owner";
const MAX_MEMBERS: i64 = 25;

pub const PERMISSIONS: &[(&str, &[&str])] = &[
    (
        "servers",
        &[
            "view", "start", "stop", "rename", "console", "install", "rebuild", "rescue",
            "restore", "rdns",
        ],
    ),
    (
        "snapshots",
        &["view", "create", "schedule", "rename", "delete"],
    ),
    ("volumes", &["view", "attach", "detach", "rename", "resize"]),
    (
        "networks",
        &[
            "view",
            "create",
            "update",
            "delete",
            "subnet_create",
            "subnet_delete",
            "attach",
            "detach",
        ],
    ),
    (
        "firewalls",
        &[
            "view",
            "create",
            "update",
            "delete",
            "rule_create",
            "rule_update",
            "rule_delete",
            "attach",
            "detach",
        ],
    ),
    ("floating_ips", &["view", "attach", "detach"]),
    ("isos", &["view", "import", "delete", "mount"]),
    ("ssh_keys", &["view", "create", "delete"]),
    ("access", &["users", "tickets"]),
];

fn known(permission: &str) -> bool {
    permission.split_once('.').is_some_and(|(group, action)| {
        PERMISSIONS
            .iter()
            .any(|(name, actions)| *name == group && actions.contains(&action))
    })
}

pub struct VpsActor {
    pub owner: Uuid,
    permissions: Option<BTreeSet<String>>,
    via_api_token: bool,
}

impl VpsActor {
    pub fn is_owner(&self) -> bool {
        self.permissions.is_none()
    }

    pub fn can(&self, permission: &str) -> bool {
        self.permissions
            .as_ref()
            .is_none_or(|granted| granted.contains(permission))
    }

    pub fn require(&self, permission: &str) -> AppResult<()> {
        if self.can(permission) {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }

    pub fn browser_only(&self) -> AppResult<()> {
        if self.via_api_token {
            return Err(AppError::Forbidden);
        }
        Ok(())
    }

    fn manage_members(&self) -> AppResult<()> {
        self.browser_only()?;
        self.require("access.users")
    }

    pub fn require_owner(&self) -> AppResult<()> {
        if self.is_owner() {
            Ok(())
        } else {
            Err(AppError::Forbidden)
        }
    }
}

impl FromRequestParts<AppState> for VpsActor {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = CurrentUser::from_request_parts(parts, state).await?;
        let owner = match parts.headers.get(OWNER_HEADER) {
            None => None,
            Some(value) => Some(
                value
                    .to_str()
                    .ok()
                    .and_then(|value| Uuid::parse_str(value.trim()).ok())
                    .ok_or(AppError::Forbidden)?,
            ),
        };
        let Some(owner) = owner.filter(|owner| *owner != user.id) else {
            return Ok(Self {
                owner: user.id,
                permissions: None,
                via_api_token: user.via_api_token,
            });
        };
        let permissions: Vec<String> = sqlx::query_scalar(
            "SELECT m.permissions FROM vps_members m JOIN users o ON o.id = m.owner_id
             WHERE m.owner_id = $1 AND m.user_id = $2 AND m.accepted_at IS NOT NULL
               AND o.deleted_at IS NULL",
        )
        .bind(owner)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Forbidden)?;
        Ok(Self {
            owner,
            permissions: Some(permissions.into_iter().collect()),
            via_api_token: user.via_api_token,
        })
    }
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
    owner_id: Uuid,
    email: String,
    name: String,
    accepted: bool,
    permissions: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize)]
pub struct Me {
    owner_id: Uuid,
    is_owner: bool,
    owner_name: String,
    permissions: Vec<String>,
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

const NAME: &str = "coalesce(nullif(trim(u.company_name), ''), nullif(trim(u.first_name || ' ' || u.last_name), ''), u.email::text)";

fn clean(actor: &VpsActor, permissions: Vec<String>) -> AppResult<Vec<String>> {
    let set: BTreeSet<String> = permissions
        .into_iter()
        .map(|p| p.trim().to_owned())
        .collect();
    if set.is_empty() {
        return Err(AppError::field("permissions", "Kies minimaal één recht."));
    }
    if let Some(unknown) = set.iter().find(|p| !known(p)) {
        return Err(AppError::field(
            "permissions",
            format!("Onbekend recht: {unknown}"),
        ));
    }
    if set.iter().any(|permission| !actor.can(permission)) {
        return Err(AppError::field(
            "permissions",
            "Je kunt alleen rechten geven die je zelf hebt.",
        ));
    }
    Ok(set.into_iter().collect())
}

async fn outranks(state: &AppState, actor: &VpsActor, id: Uuid) -> AppResult<()> {
    let current: Vec<String> =
        sqlx::query_scalar("SELECT permissions FROM vps_members WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(actor.owner)
            .fetch_optional(&state.db)
            .await?
            .ok_or(AppError::NotFound)?;
    if current.iter().all(|permission| actor.can(permission)) {
        Ok(())
    } else {
        Err(AppError::field(
            "permissions",
            "Deze gebruiker heeft rechten die jij niet hebt. Vraag de eigenaar om dit aan te passen.",
        ))
    }
}

pub async fn me(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Me>> {
    let owner_name: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT {NAME} FROM users u WHERE u.id = $1"
    )))
    .bind(actor.owner)
    .fetch_one(&state.db)
    .await?;
    let permissions = match &actor.permissions {
        Some(granted) => granted.iter().cloned().collect(),
        None => PERMISSIONS
            .iter()
            .flat_map(|(group, actions)| {
                actions
                    .iter()
                    .map(move |action| format!("{group}.{action}"))
            })
            .collect(),
    };
    Ok(Json(Me {
        owner_id: actor.owner,
        is_owner: actor.is_owner(),
        owner_name,
        permissions,
    }))
}

pub async fn index(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Vec<Member>>> {
    actor.require("access.users")?;
    Ok(Json(
        sqlx::query_as(sqlx::AssertSqlSafe(format!(
            "SELECT m.id, u.email::text AS email,
                    CASE WHEN m.accepted_at IS NULL THEN u.email::text ELSE {NAME} END AS name,
                    m.accepted_at IS NOT NULL AS accepted, m.permissions, m.created_at
             FROM vps_members m JOIN users u ON u.id = m.user_id
             WHERE m.owner_id = $1 ORDER BY m.created_at"
        )))
        .bind(actor.owner)
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    actor: VpsActor,
    Json(input): Json<NewMember>,
) -> AppResult<StatusCode> {
    actor.manage_members()?;
    state
        .limits
        .check(&crate::limits::INVITE, &actor.owner.to_string())?;
    let permissions = clean(&actor, input.permissions)?;
    let email = input.email.trim();
    if email.is_empty() {
        return Err(AppError::field("email", "Vul een e-mailadres in."));
    }
    let owner_verified: bool =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM users WHERE id = $1")
            .bind(actor.owner)
            .fetch_one(&state.db)
            .await?;
    if !owner_verified {
        return Err(AppError::field(
            "email",
            "Bevestig eerst je eigen e-mailadres voordat je iemand toegang geeft.",
        ));
    }
    let user: Option<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, email_verified_at IS NOT NULL FROM users WHERE email = $1 AND deleted_at IS NULL",
    )
    .bind(email)
    .fetch_optional(&state.db)
    .await?;
    let Some((user, verified)) = user else {
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
    if user == actor.owner {
        return Err(AppError::field(
            "email",
            "Dit is de eigenaar van deze omgeving.",
        ));
    }
    let mut transaction = state.db.begin().await?;
    crate::locks::transaction(&mut transaction, format!("members {}", actor.owner)).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM vps_members WHERE owner_id = $1")
        .bind(actor.owner)
        .fetch_one(&mut *transaction)
        .await?;
    if count >= MAX_MEMBERS {
        return Err(AppError::field(
            "email",
            "Je kunt hoogstens 25 gebruikers toegang geven.",
        ));
    }
    let inserted = sqlx::query(
        "INSERT INTO vps_members (id, owner_id, user_id, permissions) VALUES ($1, $2, $3, $4)
         ON CONFLICT (owner_id, user_id) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(actor.owner)
    .bind(user)
    .bind(&permissions)
    .execute(&mut *transaction)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(AppError::field("email", "Deze gebruiker heeft al toegang."));
    }
    transaction.commit().await?;
    let owner: String = sqlx::query_scalar("SELECT email::text FROM users WHERE id = $1")
        .bind(actor.owner)
        .fetch_one(&state.db)
        .await?;
    let _ = crate::renewals::notify(
        &state,
        user,
        (
            "Uitnodiging voor een VPS-omgeving".to_owned(),
            format!("Het account {owner} nodigt je uit voor een VPS-omgeving. Onder Gedeelde toegang kun je de uitnodiging accepteren of weigeren."),
        ),
        (
            "Invitation to a VPS environment".to_owned(),
            format!("The account {owner} invited you to their VPS environment. Accept or decline it under Shared access."),
        ),
        "/account/shared-access",
    )
    .await;
    Ok(StatusCode::CREATED)
}

pub async fn update(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
    Json(input): Json<Rights>,
) -> AppResult<StatusCode> {
    actor.manage_members()?;
    let permissions = clean(&actor, input.permissions)?;
    outranks(&state, &actor, id).await?;
    let updated = sqlx::query(
        "UPDATE vps_members SET permissions = $3, updated_at = now() WHERE id = $1 AND owner_id = $2",
    )
    .bind(id)
    .bind(actor.owner)
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
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    actor.manage_members()?;
    outranks(&state, &actor, id).await?;
    let deleted = sqlx::query("DELETE FROM vps_members WHERE id = $1 AND owner_id = $2")
        .bind(id)
        .bind(actor.owner)
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
            "SELECT m.owner_id, u.email::text AS email, {NAME} AS name,
                    m.accepted_at IS NOT NULL AS accepted, m.permissions, m.created_at
             FROM vps_members m JOIN users u ON u.id = m.owner_id
             WHERE m.user_id = $1 AND u.deleted_at IS NULL ORDER BY name"
        )))
        .bind(user.id)
        .fetch_all(&state.db)
        .await?,
    ))
}

pub async fn accept(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(owner): Path<Uuid>,
) -> AppResult<StatusCode> {
    if user.via_api_token {
        return Err(AppError::Forbidden);
    }
    let accepted = sqlx::query(
        "UPDATE vps_members m SET accepted_at = now(), updated_at = now()
         FROM users u
         WHERE m.owner_id = $1 AND m.user_id = $2 AND m.accepted_at IS NULL
           AND u.id = m.user_id AND u.email_verified_at IS NOT NULL",
    )
    .bind(owner)
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
    Path(owner): Path<Uuid>,
) -> AppResult<StatusCode> {
    let deleted = sqlx::query("DELETE FROM vps_members WHERE owner_id = $1 AND user_id = $2")
        .bind(owner)
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
    use super::{PERMISSIONS, known};

    #[test]
    fn catalogue_has_49_rights() {
        assert_eq!(
            PERMISSIONS
                .iter()
                .map(|(_, actions)| actions.len())
                .sum::<usize>(),
            49
        );
        assert!(known("servers.start"));
        assert!(!known("servers.fly"));
        assert!(!known("billing.view"));
        assert!(!known("servers"));
    }
}
