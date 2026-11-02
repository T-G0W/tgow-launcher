use serde::{Deserialize, Serialize};

use crate::manifest::{Identity, Manifest};
use crate::trust::TrustedKeys;

pub const OWNER: &str = "T-G0W";
pub const REPOSITORY_ICON: &str = "assets/logo-512.png";
pub const TOPIC: &str = "tgow-launcher";
pub const LAUNCHER_REPO: &str = "tgow-launcher";
pub const MANIFEST_FILE: &str = "tgow-release.json";
pub const LEGACY_MANIFEST_FILE: &str = "latest.json";
pub const SIGNATURE_SUFFIX: &str = ".sig";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Current,
    Legacy,
}

impl Format {
    #[must_use]
    pub fn file(self) -> &'static str {
        match self {
            Self::Current => MANIFEST_FILE,
            Self::Legacy => LEGACY_MANIFEST_FILE,
        }
    }

    fn keys(self) -> TrustedKeys {
        match self {
            Self::Current => TrustedKeys::release(),
            Self::Legacy => TrustedKeys::legacy(),
        }
    }
}

#[must_use]
pub fn evaluate(
    repository: &Repository,
    tag: &str,
    format: Format,
    manifest: &[u8],
    signature: &str,
) -> ReleaseState {
    if let Err(error) = format.keys().verify(manifest, signature) {
        return ReleaseState::Rejected(error.to_string());
    }
    let app = repository.app_id();
    let name = repository.display_name();
    match Manifest::parse(
        manifest,
        Identity {
            app: &app,
            name: &name,
        },
    ) {
        Ok(parsed) if !parsed.matches_tag(tag) => ReleaseState::Rejected(format!(
            "manifest version {} does not match tag {tag}",
            parsed.version
        )),
        Ok(parsed) if format == Format::Legacy && parsed.schema != 0 => {
            ReleaseState::Rejected("legacy file carries a current manifest".to_owned())
        }
        Ok(parsed) => ReleaseState::Verified(Box::new(parsed)),
        Err(error) => ReleaseState::Rejected(error.to_string()),
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
pub fn repository_file_url(repository: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{OWNER}/{repository}/HEAD/{path}")
}

#[must_use]
pub fn repository_banner(repository: &str) -> String {
    format!("docs/images/{}-main.png", repository.to_ascii_lowercase())
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
    use super::{
        Format, ReleaseState, Repository, download_url, evaluate, listed_repositories,
        tag_from_location,
    };

    fn ferrovox() -> Repository {
        listed_repositories(LISTING.as_bytes()).unwrap().remove(0)
    }

    const MANIFEST: &[u8] = include_bytes!("../tests/data/ferrovox-1.0.2/latest.json");
    const SIGNATURE: &str = include_str!("../tests/data/ferrovox-1.0.2/latest.json.sig");

    #[test]
    fn verifies_the_real_ferrovox_release_end_to_end() {
        let state = evaluate(&ferrovox(), "v1.0.2", Format::Legacy, MANIFEST, SIGNATURE);
        let ReleaseState::Verified(manifest) = state else {
            panic!("{state:?}");
        };
        assert_eq!(manifest.app, "ferrovox");
        assert_eq!(manifest.name, "Ferrovox");
    }

    #[test]
    fn rejects_wrong_tags_keys_and_formats() {
        assert!(matches!(
            evaluate(&ferrovox(), "v1.0.3", Format::Legacy, MANIFEST, SIGNATURE),
            ReleaseState::Rejected(_)
        ));
        assert!(matches!(
            evaluate(&ferrovox(), "v1.0.2", Format::Current, MANIFEST, SIGNATURE),
            ReleaseState::Rejected(_)
        ));
    }

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

    #[test]
    fn points_media_to_files_in_the_repository() {
        assert_eq!(
            super::repository_file_url("ferrovox", super::REPOSITORY_ICON),
            "https://raw.githubusercontent.com/T-G0W/ferrovox/HEAD/assets/logo-512.png"
        );
        assert_eq!(
            super::repository_banner("NeoCross"),
            "docs/images/neocross-main.png"
        );
    }
}
