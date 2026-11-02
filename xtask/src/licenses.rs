use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt::Write;
use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::Error;
use crate::repo::Repo;
use crate::report::Finding;

pub(crate) const LIST: &str = "assets/licenses/dependencies.txt";
pub(crate) const TEXTS: &str = "THIRD-PARTY-LICENSES.txt";

const ROOT_PACKAGE: &str = "tgow-launcher";
const PLATFORMS: [&str; 3] = [
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
];
const LICENSE_PREFIXES: [&str; 6] = [
    "LICENSE",
    "LICENCE",
    "COPYING",
    "COPYRIGHT",
    "NOTICE",
    "UNLICENSE",
];
const RULE: &str =
    "================================================================================";
const THIN_RULE: &str =
    "--------------------------------------------------------------------------------";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Crate {
    name: String,
    version: String,
    license: String,
    texts: Vec<String>,
}

pub(crate) fn run(repo: &Repo, check: bool) -> Result<Vec<Finding>, Error> {
    let crates = collect(repo)?;
    let outputs = [(LIST, list(&crates)), (TEXTS, texts(&crates))];
    let mut findings = Vec::new();
    for (path, content) in outputs {
        let target = repo.root.join(path);
        if check {
            let current = fs::read_to_string(&target).unwrap_or_default();
            if current != content {
                findings.push(Finding::whole(
                    path,
                    "out of date, run cargo xtask licenses",
                ));
            }
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, content)?;
        }
    }
    Ok(findings)
}

fn collect(repo: &Repo) -> Result<Vec<Crate>, Error> {
    let mut found = BTreeMap::new();
    for platform in PLATFORMS {
        let output = repo.run(
            "cargo",
            &[
                "metadata",
                "--format-version",
                "1",
                "--locked",
                "--filter-platform",
                platform,
            ],
        )?;
        let metadata: Value = serde_json::from_str(&output)
            .map_err(|error| Error::Command("cargo metadata".to_owned(), error.to_string()))?;
        for package in shipped(&metadata) {
            let key = (string(package, "name"), string(package, "version"));
            found.entry(key).or_insert_with(|| describe(package));
        }
    }
    Ok(found.into_values().collect())
}

fn shipped(metadata: &Value) -> Vec<&Value> {
    let packages: BTreeMap<&str, &Value> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|package| Some((package["id"].as_str()?, package)))
        .collect();
    let nodes: BTreeMap<&str, &Value> = metadata["resolve"]["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|node| Some((node["id"].as_str()?, node)))
        .collect();
    let root = packages
        .iter()
        .find(|(_, package)| package["name"] == ROOT_PACKAGE && package["source"].is_null())
        .map(|(id, _)| *id);
    let mut seen = BTreeSet::new();
    let mut queue: VecDeque<&str> = root.into_iter().collect();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        let deps = nodes
            .get(id)
            .and_then(|node| node["deps"].as_array())
            .into_iter()
            .flatten();
        for dep in deps {
            let normal = dep["dep_kinds"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|kind| kind["kind"].is_null());
            if let (true, Some(target)) = (normal, dep["pkg"].as_str()) {
                queue.push_back(target);
            }
        }
    }
    seen.into_iter()
        .filter_map(|id| packages.get(id).copied())
        .filter(|package| !package["source"].is_null())
        .collect()
}

fn describe(package: &Value) -> Crate {
    let license = package["license"]
        .as_str()
        .map(str::to_owned)
        .or_else(|| {
            package["license_file"]
                .as_str()
                .map(|_| "see license file".to_owned())
        })
        .unwrap_or_else(|| "unspecified".to_owned());
    let directory = package["manifest_path"]
        .as_str()
        .and_then(|manifest| Path::new(manifest).parent());
    Crate {
        name: string(package, "name"),
        version: string(package, "version"),
        license,
        texts: directory.map(license_texts).unwrap_or_default(),
    }
}

fn license_texts(directory: &Path) -> Vec<String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).into_iter().flatten().flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
        if path.is_dir() && name == "LICENSES" {
            files.extend(
                fs::read_dir(&path)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .map(|inner| inner.path())
                    .filter(|inner| inner.is_file()),
            );
        } else if path.is_file()
            && LICENSE_PREFIXES
                .iter()
                .any(|prefix| name.starts_with(prefix))
        {
            files.push(path);
        }
    }
    files.sort();
    files
        .iter()
        .filter_map(|path| fs::read(path).ok())
        .map(|bytes| normalize(&String::from_utf8_lossy(&bytes)))
        .filter(|text| !text.is_empty())
        .collect()
}

fn normalize(text: &str) -> String {
    let lines: Vec<&str> = text
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim_end)
        .collect();
    lines.join("\n").trim_matches('\n').to_owned()
}

fn string(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_owned()
}

fn list(crates: &[Crate]) -> String {
    let mut out = String::new();
    for krate in crates {
        let _ = writeln!(out, "{}\t{}\t{}", krate.name, krate.version, krate.license);
    }
    out
}

fn texts(crates: &[Crate]) -> String {
    let mut groups: BTreeMap<String, (&str, Vec<&Crate>)> = BTreeMap::new();
    let mut bare = Vec::new();
    for krate in crates {
        if krate.texts.is_empty() {
            bare.push(krate);
        }
        for text in &krate.texts {
            let group = groups
                .entry(fingerprint(text))
                .or_insert((text, Vec::new()));
            if !group.1.contains(&krate) {
                group.1.push(krate);
            }
        }
    }
    let mut ordered: Vec<(&str, Vec<&Crate>)> = groups.into_values().collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(right.0)));
    let mut out = String::from(
        "Third-party software in T-GOW Launcher\n\nT-GOW Launcher is built from the open-source crates listed below. Each block names\nthe crates and repeats the license text they ship with. Crates that ship no license\nfile are listed at the end with their declared license.\n",
    );
    for (text, owners) in ordered {
        out.push('\n');
        out.push_str(RULE);
        out.push('\n');
        for owner in owners {
            heading(&mut out, owner);
        }
        out.push_str(THIN_RULE);
        out.push_str("\n\n");
        out.push_str(text);
        out.push('\n');
    }
    if !bare.is_empty() {
        out.push('\n');
        out.push_str(RULE);
        out.push_str("\nCrates without a bundled license file\n");
        out.push_str(THIN_RULE);
        out.push_str("\n\n");
        for krate in bare {
            heading(&mut out, krate);
        }
    }
    out
}

fn fingerprint(text: &str) -> String {
    text.split_whitespace()
        .map(|word| word.trim_matches(['*', '"', '\'']))
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn heading(out: &mut String, krate: &Crate) {
    let _ = writeln!(out, "{} {} ({})", krate.name, krate.version, krate.license);
}

#[cfg(test)]
mod tests {
    use super::{Crate, list, normalize, texts};

    fn krate(name: &str, texts: &[&str]) -> Crate {
        Crate {
            name: name.to_owned(),
            version: "1.0.0".to_owned(),
            license: "MIT".to_owned(),
            texts: texts.iter().map(|text| (*text).to_owned()).collect(),
        }
    }

    #[test]
    fn normalizes_line_endings_and_padding() {
        assert_eq!(
            normalize("\u{feff}\r\nMIT License  \r\n\r\nPermission\r\n\r\n"),
            "MIT License\n\nPermission"
        );
    }

    #[test]
    fn groups_identical_texts_and_lists_bare_crates() {
        let crates = [
            krate("alpha", &["shared text"]),
            krate(
                "beta",
                &["shared
  text"],
            ),
            krate("gamma", &[]),
        ];
        let rendered = texts(&crates);
        assert_eq!(rendered.matches("shared").count(), 1);
        assert!(rendered.contains("alpha 1.0.0 (MIT)\nbeta 1.0.0 (MIT)\n"));
        assert!(rendered.ends_with("gamma 1.0.0 (MIT)\n"));
        assert_eq!(list(&crates[..1]), "alpha\t1.0.0\tMIT\n");
    }
}
