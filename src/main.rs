mod auth;
mod cli;
mod config;
mod crypto;
mod dates;
mod demo;
mod error;
mod html;
mod i18n;
mod imports;
mod integrations;
mod invoice_pdf;
mod mail;
mod pagination;
mod payments;
mod pdf;
mod renewals;
mod routes;
mod services;
mod settings;
mod state;
mod updates;

use anyhow::Context;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::state::AppState;

const ENV_FILE: &str = "/etc/stacklane/stacklane.env";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if dotenvy::dotenv().is_err() {
        dotenvy::from_path(ENV_FILE).ok();
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("serve");
    if command != "serve" {
        restore_default_sigpipe();
    }

    match command {
        "serve" => serve().await,
        "setup" => cli::setup::run().await,
        "update" => cli::update::run(&args[1..]).await,
        "status" => cli::status::run().await,
        "import" => cli::import::run(&args[1..]).await,
        "admin" if args.get(1).map(String::as_str) == Some("create") => {
            cli::admin::create(&args[2..]).await
        }
        "billing" if args.get(1).map(String::as_str) == Some("run") => {
            init_tracing();
            let config = Config::from_env()?;
            let state = AppState::new(connect(&config).await?, config)?;
            let (invoices, suspended, ended) = renewals::run_once(&state)
                .await
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            println!("facturen: {invoices}, opgeschort: {suspended}, beëindigd: {ended}");
            Ok(())
        }
        "demo-reset" => {
            init_tracing();
            let config = Config::from_env()?;
            demo::ensure_demo_mode(config.demo_mode)?;
            demo::reset(&connect(&config).await?).await
        }
        "version" | "--version" | "-V" => {
            println!("stacklane {}", config::VERSION);
            Ok(())
        }
        "help" | "--help" | "-h" => {
            cli::print_help();
            Ok(())
        }
        other => {
            cli::print_help();
            anyhow::bail!("onbekend commando: {other}")
        }
    }
}

fn restore_default_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("stacklane=info,tower_http=info")),
        )
        .init();
}

pub async fn connect(config: &Config) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connecting to the database")?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("running migrations")?;
    Ok(pool)
}

async fn serve() -> anyhow::Result<()> {
    init_tracing();
    let config = Config::from_env()?;
    let pool = connect(&config).await?;
    let state = AppState::new(pool, config)?;
    if state.config.demo_mode {
        let users: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
            .fetch_one(&state.db)
            .await?;
        if users == 0 {
            demo::reset(&state.db).await?;
        }
    }
    state.updates.spawn_checks();
    renewals::spawn(state.clone());
    let app = routes::router(state.clone()).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr).await?;
    tracing::info!("listening on {}", state.config.bind_addr);
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let interrupt = tokio::signal::ctrl_c();
    let mut terminate =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).ok();
    tokio::select! {
        _ = interrupt => {}
        _ = async {
            match terminate.as_mut() {
                Some(signal) => { signal.recv().await; }
                None => std::future::pending::<()>().await,
            }
        } => {}
    }
}
