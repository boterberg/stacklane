use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::Context;

use super::{
    ask, banner, bold, confirm, done, exec, fail, installed, require_root, step, wait_until_ready,
    warn,
};

const CADDY_SNIPPET: &str = "/etc/caddy/stacklane.caddy";
const CADDYFILE: &str = "/etc/caddy/Caddyfile";

fn psql(query: &str) -> anyhow::Result<String> {
    exec("runuser", &["-u", "postgres", "--", "psql", "-Atc", query])
}

fn database() -> anyhow::Result<()> {
    if !installed("psql") {
        warn("PostgreSQL staat nog niet op deze server");
        anyhow::ensure!(
            confirm("PostgreSQL nu installeren met apt?")?,
            "zonder PostgreSQL kan Stacklane niet draaien"
        );
        step("PostgreSQL installeren");
        exec("apt-get", &["install", "-y", "postgresql"])?;
    }
    exec("systemctl", &["enable", "--now", "postgresql"])?;
    let mut ready = false;
    for _ in 0..30 {
        if psql("SELECT 1").is_ok() {
            ready = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    anyhow::ensure!(
        ready,
        "PostgreSQL start niet, kijk in: journalctl -u postgresql"
    );
    if psql("SELECT 1 FROM pg_roles WHERE rolname = 'stacklane'")? != "1" {
        exec(
            "runuser",
            &["-u", "postgres", "--", "createuser", "stacklane"],
        )?;
    }
    if psql("SELECT 1 FROM pg_database WHERE datname = 'stacklane'")? != "1" {
        exec(
            "runuser",
            &[
                "-u",
                "postgres",
                "--",
                "createdb",
                "--owner",
                "stacklane",
                "stacklane",
            ],
        )?;
    }
    done("Database stacklane staat klaar");
    Ok(())
}

fn webserver(domain: &str) -> anyhow::Result<()> {
    if !installed("caddy") {
        warn("Caddy staat nog niet op deze server");
        if !confirm("Caddy installeren voor HTTPS?")? {
            warn("Sla Caddy over. Zet zelf een reverse proxy naar 127.0.0.1:8080.");
            return Ok(());
        }
        exec("apt-get", &["install", "-y", "caddy"])?;
    }

    let snippet = format!("{domain} {{\n\tencode zstd gzip\n\treverse_proxy 127.0.0.1:8080\n}}\n");
    let previous = std::fs::read_to_string(CADDYFILE).unwrap_or_default();
    std::fs::write(CADDY_SNIPPET, snippet).context("Caddy-configuratie schrijven")?;
    if !previous
        .lines()
        .any(|line| line.trim() == "import stacklane.caddy")
    {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(CADDYFILE)?;
        writeln!(file, "\nimport stacklane.caddy")?;
    }

    if let Err(error) = exec(
        "caddy",
        &["validate", "--config", CADDYFILE, "--adapter", "caddyfile"],
    ) {
        std::fs::write(CADDYFILE, previous)?;
        std::fs::remove_file(CADDY_SNIPPET).ok();
        fail("De Caddy-configuratie klopt niet, de oude is teruggezet");
        return Err(error);
    }
    exec("systemctl", &["reload-or-restart", "caddy"])?;
    done(&format!(
        "Caddy stuurt https://{domain} door naar Stacklane"
    ));
    Ok(())
}

fn set_env(key: &str, value: &str) -> anyhow::Result<()> {
    let path = "/etc/stacklane/stacklane.env";
    let current = std::fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<String> = current
        .lines()
        .filter(|line| !line.starts_with(&format!("{key}=")))
        .map(str::to_owned)
        .collect();
    lines.push(format!("{key}={value}"));
    std::fs::write(path, lines.join("\n") + "\n").context("stacklane.env bijwerken")?;
    Ok(())
}

fn admin_account() -> anyhow::Result<()> {
    let existing = exec(
        "runuser",
        &[
            "-u",
            "postgres",
            "--",
            "psql",
            "-d",
            "stacklane",
            "-Atc",
            "SELECT count(*) FROM users WHERE role = 'admin'",
        ],
    )
    .unwrap_or_default();
    if !existing.is_empty() && existing != "0" {
        done("Er is al een beheerder");
        return Ok(());
    }

    println!();
    println!("  {}", bold("Beheerdersaccount"));
    let email = ask("E-mailadres", None)?;
    let name = ask("Naam", None)?;
    let password = loop {
        let first = rpassword::prompt_password("  Wachtwoord (minstens 10 tekens): ")?;
        let second = rpassword::prompt_password("  Nog een keer: ")?;
        if first != second {
            warn("De wachtwoorden zijn niet gelijk");
        } else if first.chars().count() < 10 {
            warn("Te kort");
        } else {
            break first;
        }
    };

    let mut child = Command::new("runuser")
        .args([
            "-u",
            "stacklane",
            "--",
            "/usr/bin/stacklane",
            "admin",
            "create",
            "--email",
            &email,
            "--name",
            &name,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .context("beheerder aanmaken")?;
    if let Some(stdin) = child.stdin.as_mut() {
        writeln!(stdin, "{password}")?;
    }
    anyhow::ensure!(
        child.wait()?.success(),
        "de beheerder kon niet worden aangemaakt"
    );
    done(&format!("Beheerder {email} aangemaakt"));
    Ok(())
}

pub async fn run() -> anyhow::Result<()> {
    banner("Installatie");
    require_root()?;

    step("Database");
    database()?;

    step("Service");
    exec("systemctl", &["enable", "--now", "stacklane"])?;
    match wait_until_ready(30) {
        Some(version) => done(&format!("Stacklane {version} draait")),
        None => {
            fail("Stacklane start niet, kijk in: journalctl -u stacklane");
            anyhow::bail!("de service werd niet gereed");
        }
    }

    println!();
    step("Webadres");
    let domain = ask(
        "Op welk domein komt het panel, bijvoorbeeld panel.jouwbedrijf.nl",
        None,
    )?;
    if domain.is_empty() {
        warn("Geen domein opgegeven, de webserver sla ik over");
    } else {
        webserver(&domain)?;
        set_env("PUBLIC_URL", &format!("https://{domain}"))?;
        exec("systemctl", &["restart", "stacklane"])?;
        wait_until_ready(30);
    }

    admin_account()?;

    println!();
    if domain.is_empty() {
        done("Klaar. Stacklane luistert op http://127.0.0.1:8080");
    } else {
        done(&format!(
            "Klaar. Open https://{domain} en log in als beheerder."
        ));
    }
    if !super::apt_source_configured() {
        warn(
            "De Stacklane-pakketbron ontbreekt, updates komen dan niet binnen. Zie stacklane.io/docs/installation/install",
        );
    }
    println!();
    Ok(())
}
