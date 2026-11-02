use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tgow_core::manifest::parse_version;
use tgow_platform::storage;

pub(crate) const AFTER_UPDATE_FLAG: &str = "--after-update";
const PENDING: &str = "update-pending.json";
const ATTEMPTS_BEFORE_ROLLBACK: u32 = 2;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Pending {
    version: String,
    attempts: u32,
    skipped: Option<String>,
}

pub(crate) struct Guard {
    file: PathBuf,
    backup: PathBuf,
}

impl Guard {
    pub(crate) fn new(data: &Path, executable: &Path) -> Self {
        Self {
            file: data.join(PENDING),
            backup: backup_path(executable),
        }
    }

    fn read(&self) -> Option<Pending> {
        storage::read_text(&self.file).and_then(|text| serde_json::from_str(&text).ok())
    }

    fn write(&self, pending: &Pending) {
        if let Ok(json) = serde_json::to_string(pending) {
            let _ = storage::write_atomic(&self.file, json.as_bytes());
        }
    }

    pub(crate) fn on_start(&self, executable: &Path, current: &str) -> bool {
        let Some(mut pending) = self.read() else {
            return false;
        };
        let before = (pending.version.clone(), pending.attempts);
        match decide(&mut pending, current, self.backup.is_file()) {
            Start::Normal => {
                if (pending.version.as_str(), pending.attempts) != (before.0.as_str(), before.1) {
                    self.write(&pending);
                }
                false
            }
            Start::Rollback => {
                let rolled_back = match self_replace::self_replace(&self.backup) {
                    Ok(()) => true,
                    Err(_) => restore(executable, &self.backup),
                };
                if !rolled_back {
                    self.write(&pending);
                    return false;
                }
                pending.skipped = Some(std::mem::take(&mut pending.version));
                pending.attempts = 0;
                self.write(&pending);
                let _ = fs::remove_file(&self.backup);
                true
            }
        }
    }

    pub(crate) fn confirm(&self) {
        if let Some(mut pending) = self.read()
            && !pending.version.is_empty()
        {
            pending.version.clear();
            pending.attempts = 0;
            self.write(&pending);
            let _ = fs::remove_file(&self.backup);
        }
    }

    pub(crate) fn is_skipped(&self, version: &str) -> bool {
        self.read()
            .and_then(|pending| pending.skipped)
            .is_some_and(|skipped| skipped == version)
    }

    pub(crate) fn replace(
        &self,
        executable: &Path,
        new_executable: &Path,
        version: &str,
    ) -> io::Result<()> {
        fs::copy(executable, &self.backup)?;
        if let Err(error) = self_replace::self_replace(new_executable) {
            restore(executable, &self.backup);
            return Err(error);
        }
        let skipped = self.read().and_then(|pending| pending.skipped);
        self.write(&Pending {
            version: version.to_owned(),
            attempts: 0,
            skipped,
        });
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Start {
    Normal,
    Rollback,
}

fn decide(pending: &mut Pending, current: &str, has_backup: bool) -> Start {
    if pending.version.is_empty() {
        return Start::Normal;
    }
    if parse_version(&pending.version) != parse_version(current) {
        pending.version.clear();
        pending.attempts = 0;
        return Start::Normal;
    }
    pending.attempts += 1;
    if pending.attempts > ATTEMPTS_BEFORE_ROLLBACK && has_backup {
        Start::Rollback
    } else {
        Start::Normal
    }
}

fn restore(executable: &Path, backup: &Path) -> bool {
    !executable.is_file() && fs::copy(backup, executable).is_ok()
}

fn backup_path(executable: &Path) -> PathBuf {
    let mut name = executable
        .file_stem()
        .map(std::ffi::OsStr::to_os_string)
        .unwrap_or_default();
    name.push(".previous");
    if let Some(extension) = executable.extension() {
        name.push(".");
        name.push(extension);
    }
    executable.with_file_name(name)
}

#[must_use]
pub(crate) fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Pending, Start, backup_path, decide, is_newer};

    fn pending(version: &str, attempts: u32) -> Pending {
        Pending {
            version: version.to_owned(),
            attempts,
            skipped: None,
        }
    }

    #[test]
    fn rolls_back_only_after_repeated_starts_of_the_pending_version() {
        let mut state = pending("0.2.0", 0);
        assert_eq!(decide(&mut state, "0.2.0", true), Start::Normal);
        assert_eq!(decide(&mut state, "0.2.0", true), Start::Normal);
        assert_eq!(decide(&mut state, "0.2.0", true), Start::Rollback);
        let mut without_backup = pending("0.2.0", 2);
        assert_eq!(decide(&mut without_backup, "0.2.0", false), Start::Normal);
    }

    #[test]
    fn clears_a_marker_that_belongs_to_another_version() {
        let mut state = pending("0.2.0", 2);
        assert_eq!(decide(&mut state, "0.1.0", true), Start::Normal);
        assert_eq!(state.version, "");
        assert_eq!(state.attempts, 0);
        let mut idle = pending("", 0);
        assert_eq!(decide(&mut idle, "0.1.0", true), Start::Normal);
        assert_eq!(idle.attempts, 0);
    }

    #[test]
    fn compares_versions_semantically() {
        assert!(is_newer("v0.10.0", "0.9.3"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.1.0-beta.1", "0.1.0"));
        assert!(!is_newer("garbage", "0.1.0"));
    }

    #[test]
    fn keeps_the_backup_next_to_the_executable() {
        assert_eq!(
            backup_path(Path::new("C:/Apps/tgow-launcher.exe")),
            Path::new("C:/Apps/tgow-launcher.previous.exe")
        );
        assert_eq!(
            backup_path(Path::new("/opt/tgow-launcher")),
            Path::new("/opt/tgow-launcher.previous")
        );
    }
}
