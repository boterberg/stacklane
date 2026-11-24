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

pub fn write_progress(dir: &Path, progress: &Progress) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let temporary = dir.join("status.json.tmp");
    std::fs::write(&temporary, serde_json::to_vec(progress)?)?;
    std::fs::rename(&temporary, dir.join("status.json"))?;
    Ok(())
}

pub struct UpdateDir {
    dir: std::fs::File,
}

impl UpdateDir {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        anyhow::ensure!(
            dir.metadata()?.is_dir() && dir.metadata()?.nlink() >= 2,
            "geen map"
        );
        Ok(Self { dir })
    }

    fn entry(&self, name: &str) -> PathBuf {
        use std::os::fd::AsRawFd;
        PathBuf::from(format!("/proc/self/fd/{}/{name}", self.dir.as_raw_fd()))
    }

    pub fn take_request(&self) -> Option<String> {
        use std::io::Read;
        use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
            .open(self.entry("request"))
            .ok()?;
        let metadata = file.metadata().ok()?;
        std::fs::remove_file(self.entry("request")).ok();
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > 64 {
            return None;
        }
        let mut target = String::new();
        file.by_ref().take(64).read_to_string(&mut target).ok()?;
        Some(target)
    }

    pub fn write_progress(&self, progress: &Progress) -> anyhow::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let temporary = self.entry("status.json.tmp");
        match std::fs::remove_file(&temporary) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o644)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&temporary)?;
        file.write_all(&serde_json::to_vec(progress)?)?;
        drop(file);
        std::fs::rename(&temporary, self.entry("status.json"))?;
        Ok(())
    }
}

pub fn read_progress(dir: &Path) -> Option<Progress> {
    serde_json::from_slice(&std::fs::read(dir.join("status.json")).ok()?).ok()
}

impl Updates {
    pub fn new(dir: PathBuf, check_url: Option<String>) -> Self {
        Self {
            dir,
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
            progress: read_progress(&self.dir),
        }
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
    fn the_update_dir_never_follows_planted_links() {
        let root = scratch("links");
        let update = root.join("update");
        std::fs::create_dir(&update).unwrap();
        let secret = root.join("secret");
        std::fs::write(&secret, "root only").unwrap();
        let victim = root.join("victim");
        std::fs::write(&victim, "untouched").unwrap();
        std::os::unix::fs::symlink(&secret, update.join("request")).unwrap();
        std::os::unix::fs::symlink(&victim, update.join("status.json.tmp")).unwrap();

        let dir = UpdateDir::open(&update).unwrap();
        assert_eq!(dir.take_request(), None);
        dir.write_progress(&Progress::new("running", "checking", "1.2.3.4", None))
            .unwrap();
        assert_eq!(std::fs::read_to_string(&victim).unwrap(), "untouched");
        assert!(read_progress(&update).is_some());

        let linked = root.join("linked");
        std::os::unix::fs::symlink(&update, &linked).unwrap();
        assert!(UpdateDir::open(&linked).is_err());
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
