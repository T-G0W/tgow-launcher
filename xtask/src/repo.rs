use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::Error;

pub(crate) struct Repo {
    pub(crate) root: PathBuf,
}

pub(crate) struct SourceFile {
    pub(crate) path: String,
    pub(crate) text: Option<String>,
}

impl Repo {
    pub(crate) fn locate() -> Result<Self, Error> {
        let root = match env::var_os("CARGO_MANIFEST_DIR") {
            Some(manifest) => Path::new(&manifest)
                .parent()
                .map_or_else(|| PathBuf::from(&manifest), Path::to_path_buf),
            None => env::current_dir()?,
        };
        Ok(Self { root })
    }

    pub(crate) fn run(&self, program: &str, args: &[&str]) -> Result<String, Error> {
        let output = Command::new(program)
            .args(args)
            .current_dir(&self.root)
            .output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(Error::Command(
                format!("{program} {}", args.join(" ")),
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ))
        }
    }

    pub(crate) fn paths(&self) -> Result<Vec<String>, Error> {
        let listing = self.run(
            "git",
            &[
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
            ],
        )?;
        let mut paths: Vec<String> = listing
            .split('\0')
            .filter(|path| !path.is_empty() && self.root.join(path).is_file())
            .map(str::to_owned)
            .collect();
        paths.sort();
        paths.dedup();
        Ok(paths)
    }

    pub(crate) fn files(&self) -> Result<Vec<SourceFile>, Error> {
        self.paths()?
            .into_iter()
            .map(|path| {
                let full = self.root.join(&path);
                let bytes = fs::read(&full).map_err(|error| Error::Read(full, error))?;
                let text = String::from_utf8(bytes).ok();
                Ok(SourceFile { path, text })
            })
            .collect()
    }
}

pub(crate) fn extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .unwrap_or_default()
}

pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
