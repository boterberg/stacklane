use std::time::Duration;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::json;

const CDN: &str = "https://cdn.modrinth.com/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loader {
    Paper,
    Purpur,
    Folia,
    Spigot,
    Bukkit,
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl Loader {
    pub const ALL: [Loader; 9] = [
        Loader::Paper,
        Loader::Purpur,
        Loader::Folia,
        Loader::Spigot,
        Loader::Bukkit,
        Loader::Fabric,
        Loader::Quilt,
        Loader::Forge,
        Loader::NeoForge,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Loader::Paper => "paper",
            Loader::Purpur => "purpur",
            Loader::Folia => "folia",
            Loader::Spigot => "spigot",
            Loader::Bukkit => "bukkit",
            Loader::Fabric => "fabric",
            Loader::Quilt => "quilt",
            Loader::Forge => "forge",
            Loader::NeoForge => "neoforge",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|loader| loader.key() == value.trim())
    }

    pub fn from_egg(egg: &str) -> Option<Self> {
        let egg = egg.to_lowercase();
        [
            ("neoforge", Loader::NeoForge),
            ("forge", Loader::Forge),
            ("fabric", Loader::Fabric),
            ("quilt", Loader::Quilt),
            ("purpur", Loader::Purpur),
            ("folia", Loader::Folia),
            ("paper", Loader::Paper),
            ("spigot", Loader::Spigot),
            ("bukkit", Loader::Bukkit),
        ]
        .into_iter()
        .find(|(name, _)| egg.contains(name))
        .map(|(_, loader)| loader)
    }

    pub fn plugins(self) -> bool {
        matches!(
            self,
            Loader::Paper | Loader::Purpur | Loader::Folia | Loader::Spigot | Loader::Bukkit
        )
    }

    pub fn folder(self) -> &'static str {
        if self.plugins() { "/plugins" } else { "/mods" }
    }

    fn compatible(self) -> &'static [&'static str] {
        match self {
            Loader::Paper => &["paper", "spigot", "bukkit"],
            Loader::Purpur => &["purpur", "paper", "spigot", "bukkit"],
            Loader::Folia => &["folia"],
            Loader::Spigot => &["spigot", "bukkit"],
            Loader::Bukkit => &["bukkit"],
            Loader::Fabric => &["fabric"],
            Loader::Quilt => &["quilt", "fabric"],
            Loader::Forge => &["forge"],
            Loader::NeoForge => &["neoforge"],
        }
    }
}

pub fn minecraft_version(value: &str) -> Option<String> {
    let value = value.trim();
    let parts: Vec<&str> = value.split('.').collect();
    (matches!(parts.len(), 2 | 3)
        && parts.iter().all(|part| {
            !part.is_empty() && part.len() <= 3 && part.chars().all(|c| c.is_ascii_digit())
        }))
    .then(|| value.to_owned())
}

pub fn project_id(value: &str) -> Option<&str> {
    let value = value.trim();
    (!value.is_empty()
        && value.len() <= 64
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    .then_some(value)
}

#[derive(Serialize, Deserialize)]
pub struct Hit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub downloads: u64,
    pub icon_url: Option<String>,
    pub author: String,
}

#[derive(Deserialize)]
struct SearchReply {
    hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Version {
    version_type: String,
    files: Vec<VersionFile>,
}

#[derive(Deserialize)]
struct VersionFile {
    url: String,
    filename: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Download {
    pub url: String,
    pub filename: String,
}

#[derive(Deserialize)]
struct Project {
    title: String,
}

fn http() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(15))
        .user_agent(concat!(
            "stacklane/",
            env!("CARGO_PKG_VERSION"),
            " (+https://stacklane.io)"
        ))
        .build()?)
}

pub async fn search(
    base: &str,
    query: &str,
    loader: Loader,
    version: Option<&str>,
) -> anyhow::Result<Vec<Hit>> {
    let mut facets = vec![
        json!([format!("categories:{}", loader.key())]),
        json!(["project_type:mod", "project_type:plugin"]),
    ];
    if let Some(version) = version {
        facets.push(json!([format!("versions:{version}")]));
    }
    if !loader.plugins() {
        facets.push(json!(["server_side!=unsupported"]));
    }
    let reply: SearchReply = http()?
        .get(format!("{base}/v2/search"))
        .query(&[
            ("query", query.to_owned()),
            ("limit", "20".to_owned()),
            (
                "index",
                if query.trim().is_empty() {
                    "downloads"
                } else {
                    "relevance"
                }
                .to_owned(),
            ),
            ("facets", serde_json::to_string(&facets)?),
        ])
        .send()
        .await
        .context("Modrinth is niet bereikbaar")?
        .error_for_status()
        .context("Modrinth gaf een fout")?
        .json()
        .await?;
    Ok(reply.hits)
}

fn pick(versions: Vec<Version>) -> Option<Download> {
    let version = versions
        .iter()
        .find(|version| version.version_type == "release")
        .or_else(|| versions.first())?;
    let file = version
        .files
        .iter()
        .filter(|file| file.filename.ends_with(".jar"))
        .max_by_key(|file| file.primary)?;
    let filename = file.filename.rsplit('/').next()?.to_owned();
    let safe = filename
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'));
    (file.url.starts_with(CDN) && safe && !filename.starts_with('.')).then(|| Download {
        url: file.url.clone(),
        filename,
    })
}

pub async fn resolve(
    base: &str,
    project: &str,
    loader: Loader,
    version: Option<&str>,
) -> anyhow::Result<Option<(String, Download)>> {
    let client = http()?;
    let details: Project = client
        .get(format!("{base}/v2/project/{project}"))
        .send()
        .await
        .context("Modrinth is niet bereikbaar")?
        .error_for_status()
        .context("Modrinth kent deze plugin niet")?
        .json()
        .await?;
    let mut query = vec![("loaders", serde_json::to_string(loader.compatible())?)];
    if let Some(version) = version {
        query.push(("game_versions", serde_json::to_string(&[version])?));
    }
    let versions: Vec<Version> = client
        .get(format!("{base}/v2/project/{project}/version"))
        .query(&query)
        .send()
        .await
        .context("Modrinth is niet bereikbaar")?
        .error_for_status()
        .context("Modrinth gaf een fout")?
        .json()
        .await?;
    Ok(pick(versions).map(|download| (details.title, download)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn version(kind: &str, files: &[(&str, &str, bool)]) -> Version {
        Version {
            version_type: kind.into(),
            files: files
                .iter()
                .map(|(url, filename, primary)| VersionFile {
                    url: (*url).into(),
                    filename: (*filename).into(),
                    primary: *primary,
                })
                .collect(),
        }
    }

    #[test]
    fn loaders_come_from_the_egg_name() {
        assert_eq!(Loader::from_egg("Paper"), Some(Loader::Paper));
        assert_eq!(Loader::from_egg("NeoForge 1.21"), Some(Loader::NeoForge));
        assert_eq!(Loader::from_egg("Forge Enhanced"), Some(Loader::Forge));
        assert_eq!(Loader::from_egg("Vanilla Minecraft"), None);
        assert_eq!(Loader::Paper.folder(), "/plugins");
        assert_eq!(Loader::Fabric.folder(), "/mods");
    }

    #[test]
    fn only_plain_versions_and_ids_pass() {
        assert_eq!(minecraft_version("1.21.1").as_deref(), Some("1.21.1"));
        assert_eq!(minecraft_version("1.20").as_deref(), Some("1.20"));
        assert_eq!(minecraft_version("latest"), None);
        assert_eq!(minecraft_version("1.21\"]"), None);
        assert_eq!(project_id("P7dR8mSH"), Some("P7dR8mSH"));
        assert_eq!(project_id("../admin"), None);
        assert_eq!(project_id("a/b"), None);
    }

    #[test]
    fn downloads_come_from_the_modrinth_cdn_only() {
        let picked = pick(vec![
            version("beta", &[("https://cdn.modrinth.com/b.jar", "b.jar", true)]),
            version(
                "release",
                &[
                    (
                        "https://cdn.modrinth.com/sources.jar",
                        "x-sources.jar",
                        false,
                    ),
                    ("https://cdn.modrinth.com/x.jar", "x-1.0.jar", true),
                ],
            ),
        ]);
        assert_eq!(
            picked,
            Some(Download {
                url: "https://cdn.modrinth.com/x.jar".into(),
                filename: "x-1.0.jar".into()
            })
        );
        assert_eq!(
            pick(vec![version(
                "release",
                &[("https://evil.test/x.jar", "x.jar", true)]
            )]),
            None
        );
        assert_eq!(
            pick(vec![version(
                "release",
                &[("https://cdn.modrinth.com/x.jar", "x;y.jar", true)]
            )]),
            None
        );
        assert_eq!(
            pick(vec![version(
                "release",
                &[("https://cdn.modrinth.com/x.zip", "x.zip", true)]
            )]),
            None
        );
    }
}
