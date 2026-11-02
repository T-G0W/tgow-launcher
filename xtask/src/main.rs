#![forbid(unsafe_code)]

mod aifiles;
mod audit;
mod comments;
mod commits;
mod dist;
mod error;
mod icons;
mod licenses;
mod names;
mod repo;
mod report;

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, ExitCode};

use crate::error::Error;
use crate::repo::Repo;
use crate::report::Finding;

const USAGE: &str = "Usage: cargo xtask <command>

Commands:
  nocomments           Reject comments in every tracked file
  names                Reject catch-all type and module names
  aifiles              Reject ai tool files in the repository
  commits [range]      Check commit messages, identity and time zone
  audit-binary <file>  Reject home directory paths and email addresses in a build
  dist                 Build the release binary without local paths and audit it
  licenses [--check]   Write or verify the third-party license files
  icons                Render assets/brand/tgow-launcher.ico from the logo geometry
  precommit            Run the fast checks for the pre-commit hook
  install-hooks        Install the local pre-commit hook
";

const HOOK: &str = "#!/bin/sh\nexec cargo xtask precommit\n";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let outcome = Repo::locate().and_then(|repo| dispatch(&repo, &args));
    let mut out = io::stdout().lock();
    match outcome {
        Ok(findings) if findings.is_empty() => ExitCode::SUCCESS,
        Ok(findings) => {
            for finding in &findings {
                let _ = writeln!(out, "{finding}");
            }
            let _ = writeln!(out, "{} problem(s)", findings.len());
            ExitCode::FAILURE
        }
        Err(Error::Usage(text)) => {
            let _ = write!(io::stderr().lock(), "{text}");
            ExitCode::from(2)
        }
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(repo: &Repo, args: &[String]) -> Result<Vec<Finding>, Error> {
    let (command, rest) = args
        .split_first()
        .ok_or_else(|| Error::Usage(USAGE.to_owned()))?;
    match command.as_str() {
        "nocomments" => Ok(comments::check(&repo.files()?)),
        "names" => Ok(names::check(&repo.files()?)),
        "aifiles" => Ok(aifiles::check(&repo.paths()?)),
        "commits" => commit_check(repo, rest.first().map_or("HEAD", String::as_str)),
        "precommit" => precommit(repo),
        "install-hooks" => install_hooks(repo),
        "icons" => icons::write(&repo.root),
        "dist" => dist::build(repo),
        "licenses" => licenses::run(repo, rest.first().is_some_and(|flag| flag == "--check")),
        "audit-binary" => rest
            .first()
            .ok_or_else(|| Error::Usage(USAGE.to_owned()))
            .and_then(|file| audit::check(Path::new(file))),
        _ => Err(Error::Usage(USAGE.to_owned())),
    }
}

fn commit_check(repo: &Repo, range: &str) -> Result<Vec<Finding>, Error> {
    if repo
        .run("git", &["rev-parse", "--verify", "--quiet", "HEAD"])
        .is_err()
    {
        return Ok(Vec::new());
    }
    let log = repo.run("git", &["log", commits::LOG_FORMAT, "--date=raw", range])?;
    Ok(commits::check(&commits::parse(&log)))
}

fn precommit(repo: &Repo) -> Result<Vec<Finding>, Error> {
    let formatted = Command::new("cargo")
        .args(["fmt", "--all", "--check"])
        .current_dir(&repo.root)
        .status()?;
    let mut findings = Vec::new();
    if !formatted.success() {
        findings.push(Finding::whole("cargo fmt", "formatting differs"));
    }
    let files = repo.files()?;
    findings.extend(comments::check(&files));
    findings.extend(names::check(&files));
    let paths: Vec<String> = files.into_iter().map(|file| file.path).collect();
    findings.extend(aifiles::check(&paths));
    Ok(findings)
}

fn install_hooks(repo: &Repo) -> Result<Vec<Finding>, Error> {
    let hooks = repo.run("git", &["rev-parse", "--git-path", "hooks"])?;
    let hooks = repo.root.join(hooks.trim());
    fs::create_dir_all(&hooks)?;
    fs::write(hooks.join("pre-commit"), HOOK)?;
    Ok(Vec::new())
}
