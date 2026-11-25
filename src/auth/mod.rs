mod password;
mod session;
pub mod tokens;
pub mod totp;

pub use password::{hash, hash_password, needs_rehash, verify};
pub use session::{
    Credentials, CurrentAdmin, CurrentUser, PERMISSIONS, Staff, clear_session_cookie,
    create_session, hash_token, permission, revoke_session, session_cookie, staff_with,
};

pub async fn forget_unproven_holder(
    connection: &mut sqlx::PgConnection,
    user_id: uuid::Uuid,
) -> sqlx::Result<()> {
    for statement in [
        "DELETE FROM vps_members WHERE owner_id = $1 OR user_id = $1",
        "DELETE FROM gameserver_members WHERE user_id = $1
             OR service_id IN (SELECT id FROM services WHERE user_id = $1)",
        "DELETE FROM recovery_codes WHERE user_id = $1",
        "DELETE FROM ssh_keys WHERE user_id = $1",
        "DELETE FROM email_verifications WHERE user_id = $1",
        "UPDATE users SET totp_secret = NULL, totp_pending = NULL, totp_last_step = NULL,
             two_factor_enabled_at = NULL
         WHERE id = $1",
    ] {
        sqlx::query(statement)
            .bind(user_id)
            .execute(&mut *connection)
            .await?;
    }
    Ok(())
}

pub async fn lock_live_session(
    connection: &mut sqlx::PgConnection,
    user_id: uuid::Uuid,
    token_hash: &str,
) -> Result<(), crate::error::AppError> {
    sqlx::query_scalar::<_, i32>("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .fetch_optional(&mut *connection)
        .await?;
    let alive: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sessions
                        WHERE token_hash = $1 AND user_id = $2 AND expires_at > now())",
    )
    .bind(token_hash)
    .bind(user_id)
    .fetch_one(&mut *connection)
    .await?;
    if !alive {
        return Err(crate::error::AppError::Unauthenticated);
    }
    Ok(())
}
