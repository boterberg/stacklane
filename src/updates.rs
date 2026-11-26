use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::config::VERSION;

const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const ENABLED_UNIT: &str = "/etc/systemd/system/multi-user.target.wants/stacklane-update.path";

pub fn compare(left: &str, right: &str) -> Ordering {
    let parts = |version: &str| -> Vec<u64> {
        version
            .split(|character: char| !character.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    };
    let (mut left, mut right) = (parts(left), parts(right));
    let length = left.len().max(right.len());
    left.resize(length, 0);
    right.resize(length, 0);
    left.cmp(&right)
}

pub fn latest_in_packages(index: &str) -> Option<String> {
    index
        .split("\n\n")
        .filter(|block| {
            block
                .lines()
                .any(|line| line.trim() == "Package: stacklane")
        })
        .filter_map(|block| {
            block
                .lines()
                .find_map(|line| line.strip_prefix("Version:"))
                .map(|version| version.trim().to_owned())
        })
        .max_by(|left, right| compare(left, right))
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Progress {
    pub state: String,
    pub step: String,
    pub target: String,
    pub message: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

impl Progress {
    pub fn new(state: &str, step: &str, target: &str, message: Option<String>) -> Self {
        Self {
            state: state.to_owned(),
            step: step.to_owned(),
            target: target.to_owned(),
            message,
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state.as_str(), "queued" | "running")
            && OffsetDateTime::now_utc() - self.updated_at < time::Duration::minutes(30)
    }
}

#[derive(Clone)]
struct Latest {
    version: Option<String>,
    checked_at: OffsetDateTime,
}

pub struct Updates {
    dir: PathBuf,
    status_dir: PathBuf,
    check_url: Option<String>,
    client: reqwest::Client,
    latest: RwLock<Option<Latest>>,
}

#[derive(Serialize)]
pub struct Overview {
    pub current: &'static str,
    pub latest: Option<String>,
    pub available: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub checked_at: Option<OffsetDateTime>,
    pub can_update: bool,
    pub progress: Option<Progress>,
}

pub fn default_check_url() -> Option<String> {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "amd64",
        "aarch64" => "arm64",
        other => other,
    };
    match std::env::var("UPDATE_CHECK_URL") {
        Ok(url) if url.trim().is_empty() => None,
        Ok(url) => Some(url),
        Err(_) => Some(format!(
            "https://stacklane.io/apt/dists/stable/main/binary-{arch}/Packages"
        )),
    }
}

pub fn status_dir() -> PathBuf {
    std::env::var_os("UPDATE_STATUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/stacklane-update"))
}

pub fn write_progress(dir: &Path, progress: &Progress) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let temporary = dir.join("status.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec(progress)?)?;
    std::fs::rename(&temporary, dir.join("status.json"))?;
    Ok(())
}

pub fn read_progress(dir: &Path) -> Option<Progress> {
    serde_json::from_slice(&std::fs::read(dir.join("status.json")).ok()?).ok()
}

fn newest(left: Option<Progress>, right: Option<Progress>) -> Option<Progress> {
    match (left, right) {
        (Some(left), Some(right)) if right.updated_at >= left.updated_at => Some(right),
        (Some(left), _) => Some(left),
        (None, right) => right,
    }
}

impl Updates {
    pub fn new(dir: PathBuf, check_url: Option<String>) -> Self {
        Self::with_status_dir(dir, status_dir(), check_url)
    }

    pub fn with_status_dir(dir: PathBuf, status_dir: PathBuf, check_url: Option<String>) -> Self {
        Self {
            dir,
            status_dir,
            check_url,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(15))
                .user_agent(format!("stacklane/{VERSION}"))
                .build()
                .unwrap_or_default(),
            latest: RwLock::new(None),
        }
    }

    pub fn panel_unit_installed() -> bool {
        std::env::var("UPDATE_FROM_PANEL").is_ok_and(|value| value == "true")
            || Path::new(ENABLED_UNIT).exists()
    }

    pub async fn check(&self) -> anyhow::Result<Option<String>> {
        let Some(url) = &self.check_url else {
            return Ok(None);
        };
        let response = self.client.get(url).send().await?;
        let version = if response.status() == reqwest::StatusCode::NOT_FOUND {
            None
        } else {
            latest_in_packages(&response.error_for_status()?.text().await?)
        };
        if let Ok(mut latest) = self.latest.write() {
            *latest = Some(Latest {
                version: version.clone(),
                checked_at: OffsetDateTime::now_utc(),
            });
        }
        Ok(version)
    }

    pub fn spawn_checks(self: &std::sync::Arc<Self>) {
        if self.check_url.is_none() {
            return;
        }
        let updates = self.clone();
        tokio::spawn(async move {
            loop {
                if let Err(error) = updates.check().await {
                    tracing::warn!(%error, "checking for a new version failed");
                }
                tokio::time::sleep(CHECK_EVERY).await;
            }
        });
    }

    pub fn overview(&self) -> Overview {
        let latest = self.latest.read().ok().and_then(|latest| latest.clone());
        let version = latest.as_ref().and_then(|latest| latest.version.clone());
        Overview {
            current: VERSION,
            available: version
                .as_deref()
                .is_some_and(|version| compare(version, VERSION) == Ordering::Greater),
            latest: version,
            checked_at: latest.map(|latest| latest.checked_at),
            can_update: Self::panel_unit_installed(),
            progress: self.progress(),
        }
    }

    fn progress(&self) -> Option<Progress> {
        newest(read_progress(&self.dir), read_progress(&self.status_dir))
    }

    pub fn request(&self, target: &str) -> anyhow::Result<Progress> {
        let progress = Progress::new("queued", "queued", target, None);
        write_progress(&self.dir, &progress)?;
        std::fs::write(self.dir.join("request"), target)?;
        Ok(progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("stacklane-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_updater_reports_progress_outside_the_panel_directory() {
        let panel = scratch("panel");
        let root = scratch("root");
        let updates = Updates::with_status_dir(panel.clone(), root.clone(), None);
        updates.request("1.2.3.4").unwrap();
        assert_eq!(updates.progress().unwrap().state, "queued");

        std::thread::sleep(Duration::from_millis(5));
        write_progress(&root, &Progress::new("running", "backup", "1.2.3.4", None)).unwrap();
        assert_eq!(updates.progress().unwrap().state, "running");
        assert_eq!(read_progress(&panel).unwrap().state, "queued");

        std::fs::remove_dir_all(panel).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn compares_four_part_versions() {
        assert_eq!(compare("0.0.2.0", "0.0.1.0"), Ordering::Greater);
        assert_eq!(compare("0.0.10.0", "0.0.9.3"), Ordering::Greater);
        assert_eq!(compare("0.0.1", "0.0.1.0"), Ordering::Equal);
        assert_eq!(compare("1.0.0.0", "1.0.0.1"), Ordering::Less);
    }

    #[test]
    fn finds_the_newest_stacklane_package() {
        let index = "Package: stacklane\nVersion: 0.0.1.0\nArchitecture: amd64\n\nPackage: other\nVersion: 9.9\n\nPackage: stacklane\nVersion: 0.0.2.0\n";
        assert_eq!(latest_in_packages(index).as_deref(), Some("0.0.2.0"));
        assert_eq!(latest_in_packages(""), None);
    }
}
