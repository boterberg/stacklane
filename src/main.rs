mod activity;
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
mod invoicing;
mod limits;
mod locks;
mod mail;
mod modrinth;
mod money;
mod pagination;
mod payments;
mod pdf;
mod renewals;
mod routes;
mod services;
mod settings;
mod state;
mod updates;
mod vat;

#[cfg(test)]
mod testing;

#[cfg(test)]
mod regressions;

use anyhow::Context;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::state::AppState;

const ENV_FILE: &str = "/etc/stacklane/stacklane.env";

fn trusted_env_file(owner: u32, mode: u32, running_as: u32) -> bool {
    running_as != 0 || (owner == 0 && mode & 0o022 == 0)
}

fn load_env() {
    if cfg!(debug_assertions) && dotenvy::dotenv().is_ok() {
        return;
    }
    let path = std::env::var_os("STACKLANE_ENV_FILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(ENV_FILE));
    let Ok(metadata) = std::fs::metadata(&path) else {
        return;
    };
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let running_as = unsafe { libc::geteuid() };
    if !trusted_env_file(metadata.uid(), metadata.permissions().mode(), running_as) {
        eprintln!(
            "{} wordt overgeslagen: het bestand moet van root zijn en mag niet door anderen beschreven worden",
            path.display()
        );
        return;
    }
    dotenvy::from_path(&path).ok();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    load_env();
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
            let run = renewals::run_once(&state)
                .await
                .map_err(|error| anyhow::anyhow!("{error}"))?;
            println!(
                "facturen: {}, opgeschort: {}, beëindigd: {}, vervallen bestellingen: {}",
                run.invoices, run.suspended, run.ended, run.expired
            );
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
    services::recover_interrupted(&state).await?;
    state.updates.spawn_checks();
    renewals::spawn(state.clone());
    let app = routes::router(state.clone()).layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&state.config.bind_addr).await?;
    tracing::info!("listening on {}", state.config.bind_addr);
    if std::env::var("PUBLIC_URL").is_err() {
        tracing::warn!(
            "PUBLIC_URL is not set, so Stacklane assumes {} and refuses signed-in writes from any other address; set it in /etc/stacklane/stacklane.env",
            state.config.public_url
        );
    }
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
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

#[cfg(test)]
mod tests {
    use super::trusted_env_file;

    #[test]
    fn root_only_reads_a_root_owned_env_file() {
        assert!(trusted_env_file(0, 0o100640, 0));
        assert!(!trusted_env_file(999, 0o100640, 0));
        assert!(!trusted_env_file(0, 0o100660, 0));
        assert!(!trusted_env_file(0, 0o100646, 0));
        assert!(trusted_env_file(999, 0o100600, 999));
    }
}
