use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value, json};
use tgow_core::asset::{Kind, Name};
use tgow_core::catalog::MANIFEST_FILE;
use tgow_core::checksum::Digest;
use tgow_core::manifest::{Identity, Manifest, SCHEMA, is_app_id, is_file_name, parse_version};

use crate::options::Options;

pub(crate) fn manifest(options: &Options) -> Result<String, String> {
    let app = options.required("app")?;
    if !is_app_id(app) {
        return Err(format!("{app} is not a valid app id"));
    }
    let name = options.required("name")?;
    let version = options.required("version")?;
    parse_version(version).ok_or_else(|| format!("{version} is not a semantic version"))?;
    let channel = options.value("channel").unwrap_or("stable");
    if !matches!(channel, "stable" | "beta") {
        return Err("--channel must be stable or beta".to_owned());
    }
    if options.files.is_empty() {
        return Err("no release assets given".to_owned());
    }
    let out = options
        .value("out")
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let mut assets = Vec::new();
    let mut sums = String::new();
    for file in &options.files {
        let (entry, digest) = asset(file, app, version, options)?;
        let file_name = entry["file"].as_str().unwrap_or_default().to_owned();
        let _ = writeln!(sums, "{digest}  {file_name}");
        assets.push(entry);
    }
    let mut document = Map::new();
    document.insert("schema".to_owned(), json!(SCHEMA));
    document.insert("app".to_owned(), json!(app));
    document.insert("name".to_owned(), json!(name));
    insert_translated(
        &mut document,
        "tagline",
        options.value("tagline-de"),
        options.value("tagline-en"),
    );
    document.insert("version".to_owned(), json!(version.trim_start_matches('v')));
    document.insert("channel".to_owned(), json!(channel));
    for (key, option) in [("accent", "accent"), ("icon", "icon"), ("hero", "hero")] {
        if let Some(value) = options.value(option) {
            document.insert(key.to_owned(), json!(value));
        }
    }
    let notes_de = read_optional(options.value("notes-de"))?;
    let notes_en = read_optional(options.value("notes-en"))?;
    insert_translated(
        &mut document,
        "notes",
        notes_de.as_deref(),
        notes_en.as_deref(),
    );
    document.insert("assets".to_owned(), Value::Array(assets));
    let text = serde_json::to_string_pretty(&Value::Object(document))
        .map_err(|error| error.to_string())?;
    Manifest::parse(text.as_bytes(), Identity { app, name }).map_err(|error| error.to_string())?;
    fs::create_dir_all(&out).map_err(|error| error.to_string())?;
    let manifest_file = out.join(MANIFEST_FILE);
    fs::write(&manifest_file, &text).map_err(|error| error.to_string())?;
    fs::write(out.join("SHA256SUMS.txt"), sums).map_err(|error| error.to_string())?;
    Ok(format!("{}\n", manifest_file.display()))
}

fn asset(
    file: &Path,
    app: &str,
    version: &str,
    options: &Options,
) -> Result<(Value, Digest), String> {
    let file_name = file
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| is_file_name(name))
        .ok_or_else(|| format!("{} has no usable file name", file.display()))?;
    let parsed = Name::parse(file_name).ok_or_else(|| {
        format!("{file_name} does not follow <app>-<version>-<os>-<arch>[-portable].<ext>")
    })?;
    if parsed.app != app
        || parsed.version.trim_start_matches('v') != version.trim_start_matches('v')
    {
        return Err(format!("{file_name} does not belong to {app} {version}"));
    }
    let kind = match (parsed.kind, options.value("installer")) {
        (Some(kind), _) => kind,
        (None, Some("inno")) => Kind::Inno,
        (None, Some("nsis")) => Kind::Nsis,
        (None, _) => {
            return Err(format!(
                "{file_name} is an installer, pass --installer inno or --installer nsis"
            ));
        }
    };
    let digest = fs::File::open(file)
        .and_then(Digest::of_reader)
        .map_err(|error| format!("{file_name}: {error}"))?;
    let size = fs::metadata(file).map_err(|error| error.to_string())?.len();
    let mut entry = json!({
        "file": file_name,
        "os": parsed.os.token(),
        "arch": parsed.arch.token(parsed.os),
        "kind": kind,
        "sha256": digest.to_string(),
        "size": size,
    });
    if matches!(kind, Kind::Zip | Kind::TarGz)
        && let Some(launch) = options.value("launch")
    {
        entry["launch"] = json!(launch);
    }
    Ok((entry, digest))
}

fn insert_translated(
    document: &mut Map<String, Value>,
    key: &str,
    german: Option<&str>,
    english: Option<&str>,
) {
    let mut text = Map::new();
    if let Some(german) = german {
        text.insert("de".to_owned(), json!(german));
    }
    if let Some(english) = english {
        text.insert("en".to_owned(), json!(english));
    }
    if !text.is_empty() {
        document.insert(key.to_owned(), Value::Object(text));
    }
}

fn read_optional(file: Option<&str>) -> Result<Option<String>, String> {
    file.map(|file| fs::read_to_string(file).map_err(|error| format!("{file}: {error}")))
        .transpose()
}
