use serde::Deserialize;

use crate::manifest::Manifest;

pub const OWNER: &str = "T-G0W";
pub const TOPIC: &str = "tgow-launcher";
pub const LAUNCHER_REPO: &str = "tgow-launcher";
pub const MANIFEST_FILE: &str = "tgow-release.json";
pub const LEGACY_MANIFEST_FILE: &str = "latest.json";
pub const SIGNATURE_SUFFIX: &str = ".sig";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub fork: bool,
    #[serde(default)]
    pub private: bool,
    pub html_url: String,
    pub pushed_at: Option<String>,
}

impl Repository {
    #[must_use]
    pub fn is_listed(&self) -> bool {
        let owned = self
            .full_name
            .split_once('/')
            .is_some_and(|(owner, name)| owner.eq_ignore_ascii_case(OWNER) && name == self.name);
        owned
            && !self.archived
            && !self.fork
            && self.topics.iter().any(|topic| topic == TOPIC)
            && is_repository_name(&self.name)
    }

    #[must_use]
    pub fn app_id(&self) -> String {
        self.name.to_ascii_lowercase()
    }

    #[must_use]
    pub fn display_name(&self) -> String {
        let mut name = self.name.replace(['-', '_'], " ");
        if let Some(first) = name.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        name
    }
}

pub fn listed_repositories(json: &[u8]) -> Result<Vec<Repository>, serde_json::Error> {
    let mut repositories: Vec<Repository> = serde_json::from_slice(json)?;
    repositories.retain(Repository::is_listed);
    repositories.sort_by_key(|repository| repository.name.to_ascii_lowercase());
    Ok(repositories)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseState {
    Unknown,
    NoRelease,
    Unsigned,
    Rejected(String),
    Verified(Box<Manifest>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub id: String,
    pub repository: String,
    pub name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub tag: Option<String>,
    pub release: ReleaseState,
}

impl App {
    #[must_use]
    pub fn from_repository(repository: &Repository) -> Self {
        Self {
            id: repository.app_id(),
            repository: repository.name.clone(),
            name: repository.display_name(),
            description: repository.description.clone(),
            html_url: repository.html_url.clone(),
            tag: None,
            release: ReleaseState::Unknown,
        }
    }

    #[must_use]
    pub fn manifest(&self) -> Option<&Manifest> {
        match &self.release {
            ReleaseState::Verified(manifest) => Some(manifest),
            _ => None,
        }
    }

    #[must_use]
    pub fn title(&self) -> &str {
        self.manifest()
            .map_or(&self.name, |manifest| &manifest.name)
    }
}

#[must_use]
pub fn latest_url(repository: &str) -> String {
    format!("https://github.com/{OWNER}/{repository}/releases/latest")
}

#[must_use]
pub fn download_url(repository: &str, tag: &str, file: &str) -> String {
    format!("https://github.com/{OWNER}/{repository}/releases/download/{tag}/{file}")
}

#[must_use]
pub fn tag_from_location(repository: &str, location: &str) -> Option<String> {
    let prefix = format!("https://github.com/{OWNER}/{repository}/releases/tag/");
    let tag = location.strip_prefix(&prefix)?;
    is_tag(tag).then(|| tag.to_owned())
}

#[must_use]
pub fn is_tag(tag: &str) -> bool {
    (1..=64).contains(&tag.len())
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'+'))
}

fn is_repository_name(name: &str) -> bool {
    (1..=100).contains(&name.len())
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        && !name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::{download_url, listed_repositories, tag_from_location};

    const LISTING: &str = r#"[
        {"name":"ferrovox","full_name":"T-G0W/ferrovox","description":"Soundboard","topics":["rust","tgow-launcher"],"archived":false,"fork":false,"html_url":"https://github.com/T-G0W/ferrovox"},
        {"name":"NeoCross","full_name":"T-G0W/NeoCross","description":null,"topics":[],"archived":false,"fork":false,"html_url":"https://github.com/T-G0W/NeoCross"},
        {"name":"old-tool","full_name":"T-G0W/old-tool","topics":["tgow-launcher"],"archived":true,"fork":false,"html_url":"https://github.com/T-G0W/old-tool"},
        {"name":"forked","full_name":"T-G0W/forked","topics":["tgow-launcher"],"archived":false,"fork":true,"html_url":"https://github.com/T-G0W/forked"},
        {"name":"stranger","full_name":"someone/stranger","topics":["tgow-launcher"],"html_url":"https://github.com/someone/stranger"}
    ]"#;

    #[test]
    fn lists_only_own_active_repositories_with_the_topic() {
        let listed = listed_repositories(LISTING.as_bytes()).unwrap();
        let names: Vec<&str> = listed
            .iter()
            .map(|repository| repository.name.as_str())
            .collect();
        assert_eq!(names, vec!["ferrovox"]);
        assert_eq!(listed[0].display_name(), "Ferrovox");
    }

    #[test]
    fn reads_the_tag_from_the_latest_redirect() {
        assert_eq!(
            tag_from_location(
                "ferrovox",
                "https://github.com/T-G0W/ferrovox/releases/tag/v1.0.2"
            ),
            Some("v1.0.2".to_owned())
        );
        assert_eq!(
            tag_from_location(
                "ferrovox",
                "https://github.com/evil/ferrovox/releases/tag/v1.0.2"
            ),
            None
        );
        assert_eq!(
            tag_from_location(
                "ferrovox",
                "https://github.com/T-G0W/ferrovox/releases/tag/v1/../x"
            ),
            None
        );
    }

    #[test]
    fn builds_exact_download_urls() {
        assert_eq!(
            download_url("ferrovox", "v1.0.2", "latest.json"),
            "https://github.com/T-G0W/ferrovox/releases/download/v1.0.2/latest.json"
        );
    }
}
