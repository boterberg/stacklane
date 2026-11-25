use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::routes::vps_access::VpsActor;
use crate::state::AppState;

const KEY_TYPES: [&str; 6] = [
    "ssh-ed25519",
    "ssh-rsa",
    "ecdsa-sha2-nistp256",
    "ecdsa-sha2-nistp384",
    "ecdsa-sha2-nistp521",
    "sk-ssh-ed25519@openssh.com",
];

#[derive(Serialize, sqlx::FromRow)]
pub struct SshKey {
    id: Uuid,
    name: String,
    fingerprint: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Deserialize)]
pub struct NewKey {
    name: String,
    public_key: String,
}

pub fn parse(public_key: &str) -> Option<(String, String)> {
    let mut parts = public_key.split_whitespace();
    let (kind, body) = (parts.next()?, parts.next()?);
    if !KEY_TYPES.contains(&kind) {
        return None;
    }
    let blob = STANDARD.decode(body).ok()?;
    let length = u32::from_be_bytes(blob.get(..4)?.try_into().ok()?) as usize;
    if blob.get(4..4 + length)? != kind.as_bytes() {
        return None;
    }
    let comment: Vec<&str> = parts.collect();
    let normalized = if comment.is_empty() {
        format!("{kind} {body}")
    } else {
        format!("{kind} {body} {}", comment.join(" "))
    };
    let fingerprint = format!(
        "SHA256:{}",
        STANDARD.encode(Sha256::digest(&blob)).trim_end_matches('=')
    );
    Some((normalized, fingerprint))
}

const MAX_KEYS: i64 = 50;
const MAX_KEY_BYTES: usize = 16 * 1024;

pub async fn index(State(state): State<AppState>, actor: VpsActor) -> AppResult<Json<Vec<SshKey>>> {
    actor.require("ssh_keys.view")?;
    Ok(Json(
        sqlx::query_as("SELECT id, name, fingerprint, created_at FROM ssh_keys WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(actor.owner)
            .fetch_all(&state.db)
            .await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    actor: VpsActor,
    Json(input): Json<NewKey>,
) -> AppResult<(StatusCode, Json<SshKey>)> {
    actor.browser_only()?;
    actor.require("ssh_keys.create")?;
    let name = input.name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(AppError::field(
            "name",
            "Geef de sleutel een naam van hoogstens 60 tekens.",
        ));
    }
    if input.public_key.len() > MAX_KEY_BYTES {
        return Err(AppError::field(
            "public_key",
            "Dit is geen geldige publieke SSH-sleutel.",
        ));
    }
    let (public_key, fingerprint) = parse(input.public_key.trim()).ok_or_else(|| {
        AppError::field("public_key", "Dit is geen geldige publieke SSH-sleutel.")
    })?;
    let mut transaction = state.db.begin().await?;
    crate::locks::transaction(&mut transaction, format!("ssh-keys {}", actor.owner)).await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM ssh_keys WHERE user_id = $1")
        .bind(actor.owner)
        .fetch_one(&mut *transaction)
        .await?;
    if count >= MAX_KEYS {
        return Err(AppError::field(
            "public_key",
            "Je hebt al 50 SSH-sleutels. Verwijder er eerst een.",
        ));
    }
    let key = sqlx::query_as(
        "INSERT INTO ssh_keys (id, user_id, name, public_key, fingerprint) VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (user_id, fingerprint) DO NOTHING
         RETURNING id, name, fingerprint, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(actor.owner)
    .bind(name)
    .bind(public_key)
    .bind(fingerprint)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::field("public_key", "Deze sleutel heb je al toegevoegd."))?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(key)))
}

pub async fn destroy(
    State(state): State<AppState>,
    actor: VpsActor,
    Path(id): Path<Uuid>,
) -> AppResult<StatusCode> {
    actor.require("ssh_keys.delete")?;
    let deleted = sqlx::query("DELETE FROM ssh_keys WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(actor.owner)
        .execute(&state.db)
        .await?
        .rows_affected();
    if deleted == 0 {
        return Err(AppError::NotFound);
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn accepts_real_keys_and_computes_openssh_fingerprints() {
        let key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl jan@laptop";
        let (normalized, fingerprint) = parse(key).unwrap();
        assert_eq!(normalized, key);
        assert_eq!(
            fingerprint,
            "SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU"
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("ssh-ed25519 bm90IGEga2V5").is_none());
        assert!(parse("ssh-dss AAAAB3NzaC1kc3M=").is_none());
        assert!(parse("hallo").is_none());
    }
}
