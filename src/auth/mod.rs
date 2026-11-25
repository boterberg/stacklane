mod password;
mod session;
pub mod tokens;
pub mod totp;

pub use password::{hash, hash_password, needs_rehash, verify};
pub use session::{
    CurrentAdmin, CurrentUser, PERMISSIONS, Staff, clear_session_cookie, create_session,
    hash_token, permission, revoke_session, session_cookie, staff_with,
};

pub async fn forget_unproven_holder(
    connection: &mut sqlx::PgConnection,
    user_id: uuid::Uuid,
) -> sqlx::Result<()> {
    for statement in [
        "DELETE FROM vps_members WHERE owner_id = $1 OR user_id = $1",
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
