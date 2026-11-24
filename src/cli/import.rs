use uuid::Uuid;

use super::{banner, bold, done, run_as_service_user, step, warn};
use crate::config::Config;
use crate::imports::{self, Report};
use crate::integrations;
use crate::state::AppState;

fn print_report(report: &Report, dry_run: bool) {
    println!();
    done(&format!(
        "Klanten: {} nieuw, {} al aanwezig",
        report.users_created, report.users_matched
    ));
    done(&format!(
        "Diensten: {} nieuw, {} al aanwezig",
        report.services_created, report.services_existing
    ));
    for line in &report.skipped {
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
    } else if report.users_created > 0 {
        warn(
            "Nieuwe klanten hebben nog geen wachtwoord. Ze stellen er een in via Wachtwoord vergeten.",
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
    let report = imports::pterodactyl::run(&pool, integration_id, &client, dry_run).await?;
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
        Some(source @ ("whmcs" | "paymenter" | "blesta" | "hostbill")) => {
            anyhow::bail!("importeren uit {source} komt in een volgende versie")
        }
        _ => {
            println!("  Gebruik: stacklane import pterodactyl [--integration <naam>] [--dry-run]");
            println!();
            Ok(())
        }
    }
}
