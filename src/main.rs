mod auth;
mod config;
mod demo;
mod error;
mod html;
mod pagination;
mod routes;
mod state;

use std::sync::Arc;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("stacklane_server=info,tower_http=info")),
        )
        .init();

    let config = Config::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connecting to the database")?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("running migrations")?;

    if std::env::args().nth(1).as_deref() == Some("demo-reset") {
        demo::ensure_demo_mode(config.demo_mode)?;
        return demo::reset(&pool).await;
    }

    let state = AppState {
        db: pool,
        config: Arc::new(config),
    };
    if state.config.demo_mode {
        let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&state.db)
            .await?;
        if users == 0 {
            demo::reset(&state.db).await?;
        }
    }
    let app = routes::router(state.clone()).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr).await?;
    tracing::info!("listening on {}", state.config.bind_addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
