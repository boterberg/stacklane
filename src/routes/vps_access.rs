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
            });
        };
        let permissions: Vec<String> = sqlx::query_scalar(
            "SELECT permissions FROM vps_members WHERE owner_id = $1 AND user_id = $2",
        )
        .bind(owner)
        .bind(user.id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::Forbidden)?;
        Ok(Self {
            owner,
            permissions: Some(permissions.into_iter().collect()),
        })
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Member {
    id: Uuid,
    email: String,
    name: String,
    permissions: Vec<String>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct Shared {
    owner_id: Uuid,
    email: String,
    name: String,
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
            "SELECT m.id, u.email::text AS email, {NAME} AS name, m.permissions, m.created_at
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
    actor.require("access.users")?;
    let permissions = clean(&actor, input.permissions)?;
    let email = input.email.trim();
    if email.is_empty() {
        return Err(AppError::field("email", "Vul een e-mailadres in."));
    }
    let user: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE email = $1")
        .bind(email)
        .fetch_optional(&state.db)
        .await?;
    let Some(user) = user else {
        return Err(AppError::field(
            "email",
            "Er is geen account met dit e-mailadres. Vraag de persoon eerst een account aan te maken.",
        ));
    };
    if user == actor.owner {
        return Err(AppError::field(
            "email",
            "Dit is de eigenaar van deze omgeving.",
        ));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM vps_members WHERE owner_id = $1")
        .bind(actor.owner)
        .fetch_one(&state.db)
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
    .execute(&state.db)
    .await?;
    if inserted.rows_affected() == 0 {
        return Err(AppError::field("email", "Deze gebruiker heeft al toegang."));
    }
    let owner: String = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT {NAME} FROM users u WHERE u.id = $1"
    )))
    .bind(actor.owner)
    .fetch_one(&state.db)
    .await?;
    let _ = crate::renewals::notify(
        &state,
        user,
        (
            "Je hebt toegang tot een VPS-omgeving".to_owned(),
            format!("{owner} heeft je toegang gegeven tot zijn VPS-omgeving. Je vindt hem onder Gedeelde toegang."),
        ),
        (
            "You have access to a VPS environment".to_owned(),
            format!("{owner} gave you access to their VPS environment. You can find it under Shared access."),
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
    actor.require("access.users")?;
    let permissions = clean(&actor, input.permissions)?;
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
    actor.require("access.users")?;
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
            "SELECT m.owner_id, u.email::text AS email, {NAME} AS name, m.permissions, m.created_at
             FROM vps_members m JOIN users u ON u.id = m.owner_id
             WHERE m.user_id = $1 ORDER BY name"
        )))
        .bind(user.id)
        .fetch_all(&state.db)
        .await?,
    ))
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
