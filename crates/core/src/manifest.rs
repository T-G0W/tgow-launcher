use std::fmt;

use semver::Version;
use serde::Deserialize;

use crate::asset::{Asset, Kind};
use crate::checksum::Digest;
use crate::platform::{Arch, Os};
use crate::settings::{Channel, Language};

pub const SCHEMA: u32 = 1;
const MAX_ASSETS: usize = 64;
const MAX_TEXT: usize = 64 * 1024;
const MAX_ASSET_SIZE: u64 = 8 * 1024 * 1024 * 1024;
const MAX_NAME: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Localized {
    pub german: Option<String>,
    pub english: Option<String>,
}

impl Localized {
    #[must_use]
    pub fn get(&self, language: Language) -> Option<&str> {
        let (first, second) = match language {
            Language::German => (&self.german, &self.english),
            Language::English => (&self.english, &self.german),
        };
        first.as_deref().or(second.as_deref())
    }

    fn is_within_limit(&self) -> bool {
        [&self.german, &self.english]
            .into_iter()
            .flatten()
            .all(|text| text.len() <= MAX_TEXT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accent(pub [u8; 3]);

impl Accent {
    fn parse(text: &str) -> Option<Self> {
        let hex = text.strip_prefix('#')?;
        if hex.len() != 6 || !hex.is_ascii() {
            return None;
        }
        let channel = |range: std::ops::Range<usize>| u8::from_str_radix(hex.get(range)?, 16).ok();
        Some(Self([channel(0..2)?, channel(2..4)?, channel(4..6)?]))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub schema: u32,
    pub app: String,
    pub name: String,
    pub tagline: Localized,
    pub version: Version,
    pub channel: Channel,
    pub published: Option<String>,
    pub min_launcher: Option<Version>,
    pub category: Option<String>,
    pub accent: Option<Accent>,
    pub icon: Option<String>,
    pub hero: Option<String>,
    pub notes: Localized,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Json(String),
    UnsupportedSchema(u32),
    Invalid(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "manifest is not valid json: {error}"),
            Self::UnsupportedSchema(schema) => write!(f, "unsupported manifest schema {schema}"),
            Self::Invalid(field) => write!(f, "invalid manifest field {field}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Deserialize)]
#[serde(untagged)]
enum Text {
    Plain(String),
    Translated {
        de: Option<String>,
        en: Option<String>,
    },
}

impl From<Option<Text>> for Localized {
    fn from(text: Option<Text>) -> Self {
        match text {
            None => Self::default(),
            Some(Text::Plain(text)) => Self {
                german: Some(text.clone()),
                english: Some(text),
            },
            Some(Text::Translated { de, en }) => Self {
                german: de,
                english: en,
            },
        }
    }
}

#[derive(Deserialize)]
struct Current {
    schema: u32,
    app: String,
    name: String,
    tagline: Option<Text>,
    version: String,
    #[serde(default)]
    channel: Channel,
    published: Option<String>,
    min_launcher: Option<String>,
    category: Option<String>,
    accent: Option<String>,
    icon: Option<String>,
    hero: Option<String>,
    notes: Option<Text>,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Legacy {
    version: String,
    setup: String,
    sha256: Digest,
    size: u64,
    notes_de: Option<String>,
    notes_en: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct Identity<'a> {
    pub app: &'a str,
    pub name: &'a str,
}

impl Manifest {
    pub fn parse(bytes: &[u8], identity: Identity<'_>) -> Result<Self, Error> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|error| Error::Json(error.to_string()))?;
        let manifest = if value.get("schema").is_some() {
            Self::current(value)?
        } else {
            Self::legacy(value, identity)?
        };
        manifest.validate(identity)?;
        Ok(manifest)
    }

    fn current(value: serde_json::Value) -> Result<Self, Error> {
        let schema = value
            .get("schema")
            .and_then(serde_json::Value::as_u64)
            .and_then(|schema| u32::try_from(schema).ok())
            .ok_or(Error::Invalid("schema"))?;
        if schema != SCHEMA {
            return Err(Error::UnsupportedSchema(schema));
        }
        let raw: Current =
            serde_json::from_value(value).map_err(|error| Error::Json(error.to_string()))?;
        Ok(Self {
            schema: raw.schema,
            app: raw.app,
            name: raw.name,
            tagline: raw.tagline.into(),
            version: parse_version(&raw.version).ok_or(Error::Invalid("version"))?,
            channel: raw.channel,
            published: raw.published,
            min_launcher: raw
                .min_launcher
                .map(|version| parse_version(&version).ok_or(Error::Invalid("min_launcher")))
                .transpose()?,
            category: raw.category,
            accent: raw
                .accent
                .map(|accent| Accent::parse(&accent).ok_or(Error::Invalid("accent")))
                .transpose()?,
            icon: raw.icon,
            hero: raw.hero,
            notes: raw.notes.into(),
            assets: raw.assets,
        })
    }

    fn legacy(value: serde_json::Value, identity: Identity<'_>) -> Result<Self, Error> {
        let raw: Legacy =
            serde_json::from_value(value).map_err(|error| Error::Json(error.to_string()))?;
        if !compact(&raw.setup).starts_with(&compact(identity.app)) {
            return Err(Error::Invalid("setup"));
        }
        Ok(Self {
            schema: 0,
            app: identity.app.to_owned(),
            name: identity.name.to_owned(),
            tagline: Localized::default(),
            version: parse_version(&raw.version).ok_or(Error::Invalid("version"))?,
            channel: Channel::Stable,
            published: None,
            min_launcher: None,
            category: None,
            accent: None,
            icon: None,
            hero: None,
            notes: Localized {
                german: raw.notes_de,
                english: raw.notes_en,
            },
            assets: vec![Asset {
                file: raw.setup,
                os: Os::Windows,
                arch: Arch::X86_64,
                kind: Kind::Inno,
                sha256: raw.sha256,
                size: raw.size,
                launch: None,
                args: Vec::new(),
            }],
        })
    }

    fn validate(&self, identity: Identity<'_>) -> Result<(), Error> {
        let invalid = Error::Invalid;
        if self.app != identity.app || !is_app_id(&self.app) {
            return Err(invalid("app"));
        }
        if self.name.trim().is_empty() || self.name.len() > MAX_NAME {
            return Err(invalid("name"));
        }
        if !self.notes.is_within_limit() || !self.tagline.is_within_limit() {
            return Err(invalid("notes"));
        }
        if self.assets.is_empty() || self.assets.len() > MAX_ASSETS {
            return Err(invalid("assets"));
        }
        for file in [&self.icon, &self.hero].into_iter().flatten() {
            if !is_file_name(file) {
                return Err(invalid("icon"));
            }
        }
        for asset in &self.assets {
            if !is_file_name(&asset.file) {
                return Err(invalid("assets.file"));
            }
            if asset.size == 0 || asset.size > MAX_ASSET_SIZE {
                return Err(invalid("assets.size"));
            }
            if asset
                .launch
                .as_deref()
                .is_some_and(|launch| !is_relative_path(launch))
            {
                return Err(invalid("assets.launch"));
            }
            if asset
                .args
                .iter()
                .any(|arg| arg.len() > 256 || arg.contains('\0'))
            {
                return Err(invalid("assets.args"));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn matches_tag(&self, tag: &str) -> bool {
        parse_version(tag).is_some_and(|version| version == self.version)
    }

    pub fn asset_files(&self) -> impl Iterator<Item = &str> {
        self.assets
            .iter()
            .map(|asset| asset.file.as_str())
            .chain(self.icon.as_deref())
            .chain(self.hero.as_deref())
    }
}

#[must_use]
pub fn parse_version(text: &str) -> Option<Version> {
    let text = text.trim();
    Version::parse(text.strip_prefix('v').unwrap_or(text)).ok()
}

#[must_use]
pub fn is_app_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !id.starts_with('-')
}

#[must_use]
pub fn is_file_name(name: &str) -> bool {
    (1..=MAX_NAME).contains(&name.len())
        && name != "."
        && name != ".."
        && !name.starts_with('.')
        && name.chars().all(|c| {
            !c.is_control() && !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
        })
}

fn is_relative_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 260
        && !path.starts_with(['/', '\\'])
        && !path.contains(':')
        && path
            .split(['/', '\\'])
            .all(|part| !part.is_empty() && part != ".." && part != "." && is_file_name(part))
}

fn compact(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Error, Identity, Manifest, is_file_name, parse_version};
    use crate::asset::Kind;
    use crate::platform::{Arch, Os};
    use crate::settings::{Channel, Language};

    const OKUDRO: Identity<'static> = Identity {
        app: "okudro",
        name: "Okudro",
    };

    fn current(assets: &str) -> String {
        format!(
            r##"{{"schema":1,"app":"okudro","name":"Okudro","tagline":{{"de":"Schnell.","en":"Fast."}},"version":"1.2.0","channel":"beta","accent":"#C8FF2E","icon":"okudro-icon.png","notes":"- New","assets":[{assets}]}}"##
        )
    }

    const MSI: &str = r#"{"file":"okudro-1.2.0-windows-x64.msi","os":"windows","arch":"x64","kind":"msi","sha256":"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad","size":14680064,"launch":"okudro.exe"}"#;

    #[test]
    fn parses_schema_one() {
        let manifest = Manifest::parse(current(MSI).as_bytes(), OKUDRO).unwrap();
        assert_eq!(manifest.version, parse_version("1.2.0").unwrap());
        assert_eq!(manifest.channel, Channel::Beta);
        assert_eq!(manifest.tagline.get(Language::German), Some("Schnell."));
        assert_eq!(manifest.notes.get(Language::German), Some("- New"));
        assert_eq!(
            manifest.accent.map(|accent| accent.0),
            Some([0xC8, 0xFF, 0x2E])
        );
        let asset = &manifest.assets[0];
        assert_eq!(
            (asset.os, asset.arch, asset.kind),
            (Os::Windows, Arch::X86_64, Kind::Msi)
        );
        assert!(manifest.matches_tag("v1.2.0"));
        assert!(!manifest.matches_tag("v1.2.1"));
    }

    #[test]
    fn parses_the_real_ferrovox_manifest() {
        let bytes = include_bytes!("../tests/data/ferrovox-1.0.2/latest.json");
        let manifest = Manifest::parse(
            bytes,
            Identity {
                app: "ferrovox",
                name: "Ferrovox",
            },
        )
        .unwrap();
        assert_eq!(manifest.schema, 0);
        assert!(manifest.matches_tag("v1.0.2"));
        let asset = &manifest.assets[0];
        assert_eq!(asset.file, "Ferrovox-Setup-1.0.2.exe");
        assert_eq!(asset.kind, Kind::Inno);
        assert_eq!(asset.size, 6_675_488);
        assert!(manifest.notes.get(Language::English).is_some());
        let elsewhere = Manifest::parse(
            bytes,
            Identity {
                app: "neocross",
                name: "NeoCross",
            },
        );
        assert!(matches!(elsewhere, Err(Error::Invalid("setup"))));
    }

    #[test]
    fn rejects_unsafe_or_foreign_content() {
        let traversal = MSI.replace("okudro-1.2.0-windows-x64.msi", "../evil.msi");
        let absolute = MSI.replace("\"okudro.exe\"", "\"C:/Windows/evil.exe\"");
        let climbing = MSI.replace("\"okudro.exe\"", "\"bin/../../evil.exe\"");
        let empty = MSI.replace("14680064", "0");
        for (assets, field) in [
            (traversal.as_str(), "assets.file"),
            (absolute.as_str(), "assets.launch"),
            (climbing.as_str(), "assets.launch"),
            (empty.as_str(), "assets.size"),
        ] {
            assert_eq!(
                Manifest::parse(current(assets).as_bytes(), OKUDRO),
                Err(Error::Invalid(field))
            );
        }
        let other_app = Identity {
            app: "ferrovox",
            name: "Ferrovox",
        };
        assert_eq!(
            Manifest::parse(current(MSI).as_bytes(), other_app),
            Err(Error::Invalid("app"))
        );
    }

    #[test]
    fn rejects_future_schemas_and_bad_versions() {
        let future = current(MSI).replace("\"schema\":1", "\"schema\":2");
        assert_eq!(
            Manifest::parse(future.as_bytes(), OKUDRO),
            Err(Error::UnsupportedSchema(2))
        );
        let version = current(MSI).replace("1.2.0\",\"channel", "one\",\"channel");
        assert_eq!(
            Manifest::parse(version.as_bytes(), OKUDRO),
            Err(Error::Invalid("version"))
        );
    }

    #[test]
    fn accepts_only_plain_file_names() {
        for name in ["okudro.exe", "Ferrovox-Setup-1.0.2.exe", "a b.zip"] {
            assert!(is_file_name(name), "{name}");
        }
        for name in ["", "..", ".hidden", "a/b", "a\\b", "c:x", "a\nb"] {
            assert!(!is_file_name(name), "{name:?}");
        }
    }
}
