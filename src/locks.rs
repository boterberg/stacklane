use std::collections::HashSet;
use std::future::Future;
use std::sync::{Mutex, OnceLock};

use crate::error::{AppError, AppResult};

fn held() -> &'static Mutex<HashSet<String>> {
    static HELD: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    HELD.get_or_init(Mutex::default)
}

struct Guard(String);

impl Drop for Guard {
    fn drop(&mut self) {
        held()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(&self.0);
    }
}

pub async fn transaction(
    connection: &mut sqlx::PgConnection,
    key: impl Into<String>,
) -> sqlx::Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1))")
        .bind(key.into())
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn exclusive<T>(key: String, body: impl Future<Output = AppResult<T>>) -> AppResult<T> {
    let fresh = held()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key.clone());
    if !fresh {
        return Err(AppError::field(
            "action",
            "Hier loopt al een actie. Probeer het over een paar seconden opnieuw.",
        ));
    }
    let _guard = Guard(key);
    body.await
}

#[cfg(test)]
mod tests {
    use super::exclusive;

    #[tokio::test]
    async fn a_second_action_on_the_same_key_is_refused_until_the_first_ends() {
        let (started, wait) = tokio::sync::oneshot::channel::<()>();
        let (finish, done) = tokio::sync::oneshot::channel::<()>();
        let first = tokio::spawn(exclusive("k".into(), async move {
            started.send(()).ok();
            done.await.ok();
            Ok(())
        }));
        wait.await.unwrap();
        assert!(exclusive("k".into(), async { Ok(()) }).await.is_err());
        assert!(exclusive("other".into(), async { Ok(()) }).await.is_ok());
        finish.send(()).ok();
        first.await.unwrap().unwrap();
        assert!(exclusive("k".into(), async { Ok(()) }).await.is_ok());
    }

    #[tokio::test]
    async fn a_cancelled_action_releases_its_key() {
        let pending = exclusive::<()>("cancel".into(), std::future::pending());
        let _ = tokio::time::timeout(std::time::Duration::from_millis(10), pending).await;
        assert!(exclusive("cancel".into(), async { Ok(()) }).await.is_ok());
    }
}
