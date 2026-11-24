use anyhow::Context;
use uuid::Uuid;

use super::{banner, bold, done, run_as_service_user, step, warn};
use crate::config::Config;
use crate::imports::{self, Report};
use crate::integrations;
use crate::state::AppState;

fn print_report(report: &Report, dry_run: bool) {
    println!();
    for count in &report.counts {
        done(&format!(
            "{}: {} nieuw, {} al aanwezig",
            count.label, count.created, count.existing
        ));
    }
    for line in &report.skipped {
        warn(line);
    }
    for line in &report.notes {
        warn(line);
    }
    if dry_run {
        println!();
        println!(
            "  {}",
            bold(
                "Dit was een proef, er is niets veranderd. Laat --dry-run weg om echt te importeren."
            )
        );
    }
    println!();
}

async fn pterodactyl(args: &[String]) -> anyhow::Result<()> {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let wanted = args
        .iter()
        .position(|arg| arg == "--integration")
        .and_then(|index| args.get(index + 1))
        .cloned();

    let config = Config::from_env()?;
    let pool = crate::connect(&config).await?;
    let state = AppState::new(pool.clone(), config)?;

    let candidates: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, name FROM integrations WHERE kind = 'pterodactyl' ORDER BY created_at",
    )
    .fetch_all(&pool)
    .await?;
    let (integration_id, name) = match (&wanted, candidates.as_slice()) {
        (_, []) => anyhow::bail!(
            "er is nog geen Pterodactyl-koppeling; voeg die eerst toe onder Beheer, Koppelingen"
        ),
        (None, [only]) => only.clone(),
        (None, _) => anyhow::bail!(
            "er zijn meerdere Pterodactyl-koppelingen, kies er een met --integration: {}",
            candidates
                .iter()
                .map(|(_, name)| name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        (Some(wanted), _) => candidates
            .iter()
            .find(|(id, name)| name.eq_ignore_ascii_case(wanted) || id.to_string() == *wanted)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("geen Pterodactyl-koppeling met de naam {wanted}"))?,
    };

    step(&format!("Gebruikers en servers ophalen uit {name}"));
    let client = integrations::driver(&state, integration_id)
        .await
        .map_err(|error| anyhow::anyhow!("{error}"))?
        .into_pterodactyl()
        .map_err(|error| anyhow::anyhow!("{error}"))?;
    let mut report = imports::pterodactyl::run(&pool, integration_id, &client, dry_run).await?;
    if !dry_run
        && report
            .counts
            .iter()
            .any(|count| count.label == "Klanten" && count.created > 0)
    {
        report
            .notes
            .push("Nieuwe klanten hebben nog geen wachtwoord. Ze stellen er een in via Wachtwoord vergeten.".into());
    }
    print_report(&report, dry_run);
    Ok(())
}

fn source_url(args: &[String], env: &str) -> anyhow::Result<String> {
    let given = args
        .iter()
        .position(|arg| arg == "--database")
        .and_then(|index| args.get(index + 1))
        .cloned()
        .or_else(|| std::env::var(env).ok())
        .ok_or_else(|| {
            anyhow::anyhow!("geef de database op met --database mysql://gebruiker@host/database")
        })?;
    let mut url = url::Url::parse(&given).context("ongeldig database-adres")?;
    if url.password().is_none() {
        let password = rpassword::prompt_password(format!(
            "  Wachtwoord voor {}@{}: ",
            url.username(),
            url.host_str().unwrap_or("?")
        ))?;
        url.set_password(Some(&password))
            .map_err(|_| anyhow::anyhow!("ongeldig database-adres"))?;
    }
    Ok(url.to_string())
}

async fn whmcs(args: &[String]) -> anyhow::Result<()> {
    let dry_run = args.iter().any(|arg| arg == "--dry-run");
    let url = source_url(args, "WHMCS_DATABASE_URL")?;
    let source = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .context("kan de WHMCS-database niet bereiken")?;
    let config = Config::from_env()?;
    let pool = crate::connect(&config).await?;
    step("Klanten, producten, diensten, facturen en tickets lezen uit WHMCS");
    let report = imports::whmcs::run(&source, &pool, dry_run).await?;
    print_report(&report, dry_run);
    Ok(())
}

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    if run_as_service_user()? {
        return Ok(());
    }
    banner("Importeren");
    match args.first().map(String::as_str) {
        Some("pterodactyl") => pterodactyl(&args[1..]).await,
        Some("whmcs") => whmcs(&args[1..]).await,
        Some(source @ ("paymenter" | "blesta" | "hostbill")) => {
            anyhow::bail!("importeren uit {source} komt in een volgende versie")
        }
        _ => {
            println!("  Gebruik:");
            println!("    stacklane import pterodactyl [--integration <naam>] [--dry-run]");
            println!(
                "    stacklane import whmcs --database mysql://gebruiker@host/database [--dry-run]"
            );
            println!();
            Ok(())
        }
    }
}
