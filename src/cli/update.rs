use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::Context;

use super::{banner, done, exec, fail, require_root, step, wait_until_ready, warn};
use crate::config::VERSION;
use crate::updates::{Progress, status_dir, write_progress};

const BACKUP_DIR: &str = "/var/backups/stacklane";

fn candidate_version() -> anyhow::Result<Option<String>> {
    let policy = exec("apt-cache", &["policy", "stacklane"])?;
    Ok(policy
        .lines()
        .find_map(|line| line.trim().strip_prefix("Candidate:"))
        .map(|value| value.trim().to_owned())
        .filter(|value| value != "(none)"))
}

fn private_file(dir: &std::path::Path, name: &str) -> anyhow::Result<(PathBuf, std::fs::File)> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))?;
    let path = dir.join(name);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(&path)
        .context("back-upbestand maken")?;
    Ok((path, file))
}

fn backup() -> anyhow::Result<String> {
    let stamp = exec("date", &["+%Y%m%d-%H%M%S"])?;
    let (path, file) = private_file(
        std::path::Path::new(BACKUP_DIR),
        &format!("stacklane-{VERSION}-{stamp}.dump"),
    )?;
    let path = path.display().to_string();
    let status = super::clean(&mut Command::new("runuser"))
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
    if !status.success() {
        std::fs::remove_file(&path).ok();
        anyhow::bail!("de back-up is mislukt, er is niets bijgewerkt");
    }
    Ok(path)
}

struct Panel {
    dir: PathBuf,
}

impl Panel {
    fn report(&self, state: &str, step: &str, target: &str, message: Option<String>) {
        let progress = Progress::new(state, step, target, message);
        if let Err(error) = write_progress(&self.dir, &progress) {
            warn(&format!("Status voor het panel niet opgeslagen: {error}"));
        }
    }
}

pub async fn run(args: &[String]) -> anyhow::Result<()> {
    if !args.iter().any(|arg| arg == "--from-panel") {
        return update(None);
    }
    let panel = Panel { dir: status_dir() };
    panel.report("running", "checking", "", None);
    let result = update(Some(&panel));
    if let Err(error) = &result {
        panel.report("failed", "failed", "", Some(error.to_string()));
    }
    result
}

fn update(panel: Option<&Panel>) -> anyhow::Result<()> {
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
    let report = |state: &str, step: &str| {
        if let Some(panel) = panel {
            panel.report(state, step, &candidate, None);
        }
    };
    if candidate == VERSION {
        done(&format!("Je draait al de nieuwste versie ({VERSION})"));
        report("done", "done");
        return Ok(());
    }
    done(&format!(
        "Versie {candidate} staat klaar, je draait nu {VERSION}"
    ));

    report("running", "backup");
    step("Back-up van de database");
    let path = backup()?;
    done(&format!("Opgeslagen in {path}"));

    report("running", "installing");
    step(&format!("Stacklane {candidate} installeren"));
    let status = Command::new("apt-get")
        .args(["install", "-y", "--only-upgrade", "stacklane"])
        .env("DEBIAN_FRONTEND", "noninteractive")
        .stdout(Stdio::null())
        .status()
        .context("apt-get starten")?;
    anyhow::ensure!(status.success(), "apt-get install is mislukt");

    report("running", "restarting");
    match wait_until_ready(30) {
        Some(version) => done(&format!("Stacklane {version} draait")),
        None => {
            fail("De nieuwe versie start niet. Kijk in: journalctl -u stacklane");
            warn(&format!("De back-up staat in {path}"));
            anyhow::bail!("de nieuwe versie start niet, de back-up staat in {path}");
        }
    }
    report("done", "done");
    println!();
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::private_file;

    #[test]
    fn backups_are_only_readable_by_root() {
        let dir = std::env::temp_dir()
            .join(format!("stacklane-backup-{}", uuid::Uuid::new_v4()))
            .join("backups");
        let (path, _) = private_file(&dir, "test.dump").unwrap();
        let mode =
            |path: &std::path::Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(mode(&path), 0o600);
        assert!(private_file(&dir, "test.dump").is_err());
        std::fs::remove_dir_all(dir.parent().unwrap()).unwrap();
    }
}
