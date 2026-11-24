use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::Context;

use super::{banner, done, exec, fail, require_root, step, wait_until_ready, warn};
use crate::config::VERSION;
use crate::updates::{Progress, write_progress};

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

struct Panel {
    dir: PathBuf,
    target: String,
}

impl Panel {
    fn report(&self, state: &str, step: &str, message: Option<String>) {
        let progress = Progress::new(state, step, &self.target, message);
        if let Err(error) = write_progress(&self.dir, &progress) {
            warn(&format!("Status voor het panel niet opgeslagen: {error}"));
        }
    }
}

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    if !args.iter().any(|arg| arg == "--from-panel") {
        return update(None);
    }
    let dir = std::env::var_os("UPDATE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/stacklane/update"));
    let target = std::fs::read_to_string(dir.join("request")).unwrap_or_default();
    std::fs::remove_file(dir.join("request")).ok();
    let panel = Panel {
        dir,
        target: target.trim().to_owned(),
    };
    panel.report("running", "checking", None);
    let result = update(Some(&panel));
    match &result {
        Ok(()) => panel.report("done", "done", None),
        Err(error) => panel.report("failed", "failed", Some(error.to_string())),
    }
    result
}

fn update(panel: Option<&Panel>) -> anyhow::Result<()> {
    let report = |step: &str| {
        if let Some(panel) = panel {
            panel.report("running", step, None);
        }
    };
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

    report("backup");
    step("Back-up van de database");
    let path = backup()?;
    done(&format!("Opgeslagen in {path}"));

    report("installing");
    step(&format!("Stacklane {candidate} installeren"));
    let status = Command::new("apt-get")
        .args(["install", "-y", "--only-upgrade", "stacklane"])
        .env("DEBIAN_FRONTEND", "noninteractive")
        .stdout(Stdio::null())
        .status()
        .context("apt-get starten")?;
    anyhow::ensure!(status.success(), "apt-get install is mislukt");

    report("restarting");
    match wait_until_ready(30) {
        Some(version) => done(&format!("Stacklane {version} draait")),
        None => {
            fail("De nieuwe versie start niet. Kijk in: journalctl -u stacklane");
            warn(&format!("De back-up staat in {path}"));
            anyhow::bail!("de nieuwe versie start niet, de back-up staat in {path}");
        }
    }
    println!();
    Ok(())
}
