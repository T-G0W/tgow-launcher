use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::audit;
use crate::error::Error;
use crate::repo::Repo;
use crate::report::Finding;

const PACKAGE: &str = "tgow-launcher";
const SEPARATOR: char = '\u{1f}';

pub(crate) fn build(repo: &Repo) -> Result<Vec<Finding>, Error> {
    let flags = rustflags(&repo.root, &cargo_home());
    let status = Command::new("cargo")
        .args(["build", "--release", "--locked", "--package", PACKAGE])
        .env("CARGO_ENCODED_RUSTFLAGS", flags)
        .env_remove("RUSTFLAGS")
        .current_dir(&repo.root)
        .status()?;
    if !status.success() {
        return Err(Error::Command("cargo build".to_owned(), status.to_string()));
    }
    let binary = repo
        .root
        .join("target")
        .join("release")
        .join(format!("{PACKAGE}{}", env::consts::EXE_SUFFIX));
    let findings = audit::check(&binary)?;
    if findings.is_empty() {
        let _ = writeln!(io::stdout().lock(), "{}", binary.display());
    }
    Ok(findings)
}

fn cargo_home() -> PathBuf {
    env::var_os("CARGO_HOME").map_or_else(
        || {
            let home = env::var_os("USERPROFILE")
                .or_else(|| env::var_os("HOME"))
                .unwrap_or_default();
            Path::new(&home).join(".cargo")
        },
        PathBuf::from,
    )
}

fn rustflags(root: &Path, cargo_home: &Path) -> String {
    let mut flags = vec![
        format!("--remap-path-prefix={}=/cargo", cargo_home.display()),
        format!("--remap-path-prefix={}=/src", root.display()),
    ];
    if cfg!(target_env = "msvc") {
        flags.push("-Ctarget-feature=+crt-static".to_owned());
    }
    flags.join(&SEPARATOR.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::rustflags;

    #[test]
    fn remaps_cargo_home_and_workspace() {
        let flags = rustflags(Path::new("/work/launcher"), Path::new("/opt/cargo"));
        let parts: Vec<&str> = flags.split('\u{1f}').collect();
        assert_eq!(parts[0], "--remap-path-prefix=/opt/cargo=/cargo");
        assert_eq!(parts[1], "--remap-path-prefix=/work/launcher=/src");
    }
}
