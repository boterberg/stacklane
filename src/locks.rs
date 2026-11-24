use std::future::Future;

use sqlx::Connection;

use crate::error::AppResult;
use crate::state::AppState;

pub async fn exclusive<T>(
    state: &AppState,
    key: String,
    body: impl Future<Output = AppResult<T>>,
) -> AppResult<T> {
    let mut connection = state.db.acquire().await?.detach();
    sqlx::query("SELECT pg_advisory_lock(hashtext($1::text))")
        .bind(&key)
        .execute(&mut connection)
        .await?;
    let result = body.await;
    let released = sqlx::query("SELECT pg_advisory_unlock(hashtext($1::text))")
        .bind(&key)
        .execute(&mut connection)
        .await;
    connection.close().await.ok();
    released?;
    result
}
