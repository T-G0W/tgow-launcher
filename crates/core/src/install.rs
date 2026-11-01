use std::path::PathBuf;

use semver::Version;
use serde::{Deserialize, Serialize};

use crate::asset::Kind;
use crate::manifest::{Manifest, parse_version};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Removal {
    Folder(PathBuf),
    Command { program: PathBuf, args: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Origin {
    Launcher,
    Detected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Installation {
    pub app: String,
    pub version: String,
    pub kind: Kind,
    pub location: PathBuf,
    pub launch: Option<PathBuf>,
    pub removal: Option<Removal>,
    pub origin: Origin,
}

impl Installation {
    #[must_use]
    pub fn semver(&self) -> Option<Version> {
        parse_version(&self.version)
    }

    #[must_use]
    pub fn is_outdated(&self, manifest: &Manifest) -> bool {
        self.semver()
            .is_none_or(|installed| installed < manifest.version)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Ledger {
    pub installations: Vec<Installation>,
}

impl Ledger {
    #[must_use]
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    #[must_use]
    pub fn get(&self, app: &str) -> Option<&Installation> {
        self.installations
            .iter()
            .find(|installation| installation.app == app)
    }

    pub fn record(&mut self, installation: Installation) {
        self.installations
            .retain(|known| known.app != installation.app);
        self.installations.push(installation);
        self.installations.sort_by(|a, b| a.app.cmp(&b.app));
    }

    pub fn forget(&mut self, app: &str) -> Option<Installation> {
        let index = self
            .installations
            .iter()
            .position(|known| known.app == app)?;
        Some(self.installations.remove(index))
    }
}

#[must_use]
pub fn split_command(line: &str) -> Option<(PathBuf, Vec<String>)> {
    let line = line.trim();
    let (program, rest) = if let Some(quoted) = line.strip_prefix('"') {
        let end = quoted.find('"')?;
        (&quoted[..end], &quoted[end + 1..])
    } else {
        line.split_once(' ').unwrap_or((line, ""))
    };
    if program.is_empty() {
        return None;
    }
    let args = rest.split_whitespace().map(str::to_owned).collect();
    Some((PathBuf::from(program), args))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Installation, Ledger, Origin, split_command};
    use crate::asset::Kind;

    fn ferrovox(version: &str) -> Installation {
        Installation {
            app: "ferrovox".to_owned(),
            version: version.to_owned(),
            kind: Kind::Inno,
            location: PathBuf::from("Ferrovox"),
            launch: None,
            removal: None,
            origin: Origin::Detected,
        }
    }

    #[test]
    fn splits_quoted_and_plain_uninstall_commands() {
        let (program, args) =
            split_command(r#""C:\Programs\Ferrovox\unins000.exe" /SILENT"#).unwrap();
        assert_eq!(program, PathBuf::from(r"C:\Programs\Ferrovox\unins000.exe"));
        assert_eq!(args, vec!["/SILENT".to_owned()]);
        let (program, args) = split_command("MsiExec.exe /X{1234}").unwrap();
        assert_eq!(program, PathBuf::from("MsiExec.exe"));
        assert_eq!(args, vec!["/X{1234}".to_owned()]);
        assert!(split_command("\"unterminated").is_none());
        assert!(split_command("   ").is_none());
    }

    #[test]
    fn ledger_keeps_one_entry_per_app() {
        let mut ledger = Ledger::default();
        ledger.record(ferrovox("1.0.1"));
        ledger.record(ferrovox("1.0.2"));
        assert_eq!(ledger.installations.len(), 1);
        assert_eq!(
            ledger.get("ferrovox").map(|known| known.version.as_str()),
            Some("1.0.2")
        );
        assert!(
            Ledger::from_json(&ledger.to_json())
                .get("ferrovox")
                .is_some()
        );
        assert!(ledger.forget("ferrovox").is_some());
        assert!(ledger.get("ferrovox").is_none());
    }
}
