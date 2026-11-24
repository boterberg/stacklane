use std::process::{Command, Stdio};

use anyhow::Context;

use super::{banner, done, exec, fail, require_root, step, wait_until_ready, warn};
use crate::config::VERSION;

const BACKUP_DIR: &str = "/var/backups/stacklane";

fn candidate_version() -> anyhow::Result<Option<String>> {
    let policy = exec("apt-cache", &["policy", "stacklane"])?;
    Ok(policy
        .lines()
        .find_map(|line| line.trim().strip_prefix("Candidate:"))
        .map(|value| value.trim().to_owned())
        .filter(|value| value != "(none)"))
}

fn backup() -> anyhow::Result<String> {
    std::fs::create_dir_all(BACKUP_DIR)?;
    let stamp = exec("date", &["+%Y%m%d-%H%M%S"])?;
    let path = format!("{BACKUP_DIR}/stacklane-{VERSION}-{stamp}.dump");
    let file = std::fs::File::create(&path).context("back-upbestand maken")?;
    let status = Command::new("runuser")
        .args([
            "-u",
            "postgres",
            "--",
            "pg_dump",
            "--format=custom",
            "stacklane",
        ])
        .stdout(Stdio::from(file))
        .status()
        .context("pg_dump starten")?;
    anyhow::ensure!(
        status.success(),
        "de back-up is mislukt, er is niets bijgewerkt"
    );
    exec("chmod", &["600", &path])?;
    Ok(path)
}

pub async fn run() -> anyhow::Result<()> {
    banner("Bijwerken");
    require_root()?;

    step("Pakketbron bijwerken");
    exec("apt-get", &["update", "-qq"])?;
    let Some(candidate) = candidate_version()? else {
        fail(
            "De Stacklane-pakketbron is niet ingesteld, zie stacklane.io/docs/installation/install",
        );
        anyhow::bail!("geen pakketbron");
    };
    if candidate == VERSION {
        done(&format!("Je draait al de nieuwste versie ({VERSION})"));
        return Ok(());
    }
    done(&format!(
        "Versie {candidate} staat klaar, je draait nu {VERSION}"
    ));

    step("Back-up van de database");
    let path = backup()?;
    done(&format!("Opgeslagen in {path}"));

    step(&format!("Stacklane {candidate} installeren"));
    let status = Command::new("apt-get")
        .args(["install", "-y", "--only-upgrade", "stacklane"])
        .env("DEBIAN_FRONTEND", "noninteractive")
        .stdout(Stdio::null())
        .status()
        .context("apt-get starten")?;
    anyhow::ensure!(status.success(), "apt-get install is mislukt");

    match wait_until_ready(30) {
        Some(version) => done(&format!("Stacklane {version} draait")),
        None => {
            fail("De nieuwe versie start niet. Kijk in: journalctl -u stacklane");
            warn(&format!("De back-up staat in {path}"));
            anyhow::bail!("de service werd niet gereed");
        }
    }
    println!();
    Ok(())
}
