use std::fmt;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};

use tgow_core::asset::{Asset, Kind};
use tgow_core::install::{Installation, Origin, Removal};

use crate::process;

const MAX_ENTRIES: usize = 10_000;
const MAX_UNPACKED: u64 = 4 * 1024 * 1024 * 1024;
const INNO_SILENT: [&str; 4] = ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/SP-"];

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Archive(String),
    Exit(Option<i32>),
    AnotherInstaller,
    CancelledByUser,
    NotDetected,
    Unsupported(Kind),
    MissingLaunch,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Archive(reason) => write!(f, "archive rejected: {reason}"),
            Self::Exit(Some(code)) => write!(f, "installer ended with code {code}"),
            Self::Exit(None) => f.write_str("installer was terminated"),
            Self::AnotherInstaller => f.write_str("another installation is running"),
            Self::CancelledByUser => f.write_str("installation was cancelled"),
            Self::NotDetected => f.write_str("installed app could not be found afterwards"),
            Self::Unsupported(kind) => {
                write!(f, "{} packages are not supported here", kind.label())
            }
            Self::MissingLaunch => f.write_str("no program to start was found in the package"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug, Clone)]
pub struct Target<'a> {
    pub app: &'a str,
    pub name: &'a str,
    pub version: &'a str,
    pub root: &'a Path,
}

impl Target<'_> {
    fn folder(&self) -> PathBuf {
        self.root.join(self.app).join(self.version)
    }
}

pub fn install(asset: &Asset, file: &Path, target: &Target<'_>) -> Result<Installation, Error> {
    match asset.kind {
        Kind::Exe | Kind::AppImage => portable_file(asset, file, target),
        Kind::Zip => unpacked(asset, target, |folder| extract_zip(file, folder)),
        Kind::TarGz => unpacked(asset, target, |folder| extract_tar(file, folder)),
        Kind::Inno => system(
            asset,
            target,
            file,
            INNO_SILENT.iter().copied().chain(["/CURRENTUSER"]),
        ),
        Kind::Nsis => system(asset, target, file, ["/S"]),
        Kind::Msi => {
            let file = file.to_string_lossy().into_owned();
            let status = process::run(
                Path::new("msiexec"),
                [
                    "/i",
                    file.as_str(),
                    "/qn",
                    "/norestart",
                    "ALLUSERS=2",
                    "MSIINSTALLPERUSER=1",
                ]
                .into_iter()
                .map(str::to_owned)
                .chain(asset.args.iter().cloned()),
            )?;
            check_msi(status)?;
            detected(target, asset.kind)
        }
        Kind::Flatpak => {
            let file = file.to_string_lossy().into_owned();
            let status = process::run(
                Path::new("flatpak"),
                [
                    "install",
                    "--user",
                    "--noninteractive",
                    "--bundle",
                    file.as_str(),
                ]
                .map(str::to_owned),
            )?;
            if status != Some(0) {
                return Err(Error::Exit(status));
            }
            Ok(Installation {
                app: target.app.to_owned(),
                version: target.version.to_owned(),
                kind: Kind::Flatpak,
                location: PathBuf::new(),
                launch: None,
                removal: None,
                origin: Origin::Launcher,
            })
        }
        kind => Err(Error::Unsupported(kind)),
    }
}

pub fn uninstall(installation: &Installation) -> Result<(), Error> {
    match &installation.removal {
        Some(Removal::Folder(folder)) => {
            if folder.exists() {
                fs::remove_dir_all(folder)?;
            }
            if let Some(app_folder) = folder.parent()
                && app_folder
                    .read_dir()
                    .is_ok_and(|mut entries| entries.next().is_none())
            {
                let _ = fs::remove_dir(app_folder);
            }
            Ok(())
        }
        Some(Removal::Command { program, args }) => {
            let silent: Vec<String> = match installation.kind {
                Kind::Inno => INNO_SILENT[..3]
                    .iter()
                    .map(|arg| (*arg).to_owned())
                    .collect(),
                Kind::Nsis => vec!["/S".to_owned()],
                Kind::Msi => args
                    .iter()
                    .map(|arg| arg.replace("/I{", "/X{"))
                    .chain(["/qn".to_owned(), "/norestart".to_owned()])
                    .collect(),
                _ => args.clone(),
            };
            let status = process::run(program, silent)?;
            if matches!(status, Some(0 | 3010 | 1641)) {
                Ok(())
            } else {
                Err(Error::Exit(status))
            }
        }
        None => Err(Error::Unsupported(installation.kind)),
    }
}

fn portable_file(asset: &Asset, file: &Path, target: &Target<'_>) -> Result<Installation, Error> {
    let folder = target.folder();
    fs::create_dir_all(&folder)?;
    let destination = folder.join(&asset.file);
    fs::copy(file, &destination)?;
    process::make_executable(&destination)?;
    Ok(portable(target, asset.kind, folder, destination))
}

fn unpacked(
    asset: &Asset,
    target: &Target<'_>,
    extract: impl FnOnce(&Path) -> Result<(), Error>,
) -> Result<Installation, Error> {
    let folder = target.folder();
    let staging = folder.with_extension("partial");
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    if let Err(error) = extract(&staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if folder.exists() {
        fs::remove_dir_all(&folder)?;
    }
    fs::rename(&staging, &folder)?;
    let launch = match &asset.launch {
        Some(relative) => folder.join(relative),
        None => single_program(&folder).ok_or(Error::MissingLaunch)?,
    };
    if !launch.is_file() {
        return Err(Error::MissingLaunch);
    }
    process::make_executable(&launch)?;
    Ok(portable(target, asset.kind, folder, launch))
}

fn portable(target: &Target<'_>, kind: Kind, folder: PathBuf, launch: PathBuf) -> Installation {
    Installation {
        app: target.app.to_owned(),
        version: target.version.to_owned(),
        kind,
        location: folder.clone(),
        launch: Some(launch),
        removal: Some(Removal::Folder(folder)),
        origin: Origin::Launcher,
    }
}

fn system<'a>(
    asset: &Asset,
    target: &Target<'_>,
    file: &Path,
    silent: impl IntoIterator<Item = &'a str>,
) -> Result<Installation, Error> {
    let status = process::run(
        file,
        silent
            .into_iter()
            .map(str::to_owned)
            .chain(asset.args.iter().cloned()),
    )?;
    if status != Some(0) {
        return Err(Error::Exit(status));
    }
    detected(target, asset.kind)
}

fn detected(target: &Target<'_>, kind: Kind) -> Result<Installation, Error> {
    let mut installation =
        crate::detect_installation(target.app, target.name).ok_or(Error::NotDetected)?;
    installation.origin = Origin::Launcher;
    installation.kind = kind;
    Ok(installation)
}

fn check_msi(status: Option<i32>) -> Result<(), Error> {
    match status {
        Some(0 | 3010 | 1641) => Ok(()),
        Some(1618) => Err(Error::AnotherInstaller),
        Some(1602) => Err(Error::CancelledByUser),
        other => Err(Error::Exit(other)),
    }
}

fn single_program(folder: &Path) -> Option<PathBuf> {
    let extension = if cfg!(windows) { "exe" } else { "" };
    let mut candidates: Vec<PathBuf> = fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path.extension().map_or(extension.is_empty(), |found| {
                    found.eq_ignore_ascii_case(extension)
                })
        })
        .collect();
    (candidates.len() == 1).then(|| candidates.remove(0))
}

fn safe_join(root: &Path, relative: &Path) -> Option<PathBuf> {
    let mut path = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (path != root).then_some(path)
}

fn extract_zip(file: &Path, folder: &Path) -> Result<(), Error> {
    let archive_error = |error: zip::result::ZipError| Error::Archive(error.to_string());
    let mut archive = zip::ZipArchive::new(File::open(file)?).map_err(archive_error)?;
    if archive.len() > MAX_ENTRIES {
        return Err(Error::Archive("too many entries".to_owned()));
    }
    let mut unpacked = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        let shown = entry
            .name()
            .map(std::borrow::Cow::into_owned)
            .unwrap_or_default();
        let relative = entry
            .enclosed_name()
            .ok_or_else(|| Error::Archive(format!("unsafe path {shown}")))?;
        let path = safe_join(folder, &relative)
            .ok_or_else(|| Error::Archive(format!("unsafe path {shown}")))?;
        if entry.is_symlink() {
            return Err(Error::Archive(format!("symbolic link {shown}")));
        }
        if entry.is_dir() {
            fs::create_dir_all(&path)?;
            continue;
        }
        unpacked = unpacked.saturating_add(entry.size());
        if unpacked > MAX_UNPACKED {
            return Err(Error::Archive("unpacked size too large".to_owned()));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        io::copy(&mut entry, &mut File::create(&path)?)?;
    }
    Ok(())
}

fn extract_tar(file: &Path, folder: &Path) -> Result<(), Error> {
    let decoder = flate2::read::GzDecoder::new(File::open(file)?);
    let mut archive = tar::Archive::new(decoder);
    let mut unpacked = 0_u64;
    for (count, entry) in archive.entries()?.enumerate() {
        if count >= MAX_ENTRIES {
            return Err(Error::Archive("too many entries".to_owned()));
        }
        let mut entry = entry?;
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir()) {
            return Err(Error::Archive(
                "links and special files are not allowed".to_owned(),
            ));
        }
        let relative = entry.path()?.into_owned();
        let path = safe_join(folder, &relative)
            .ok_or_else(|| Error::Archive(format!("unsafe path {}", relative.display())))?;
        unpacked = unpacked.saturating_add(entry.size());
        if unpacked > MAX_UNPACKED {
            return Err(Error::Archive("unpacked size too large".to_owned()));
        }
        if kind.is_dir() {
            fs::create_dir_all(&path)?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            entry.unpack(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};

    use tgow_core::asset::{Asset, Kind};
    use tgow_core::checksum::Digest;
    use tgow_core::install::Removal;
    use tgow_core::platform::{Arch, Os};

    use super::{Error, Target, install, safe_join, uninstall};

    fn scratch(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("tgow-install-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    fn zip_with(path: &Path, entries: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, data) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    fn asset(kind: Kind, launch: Option<&str>) -> Asset {
        Asset {
            file: "tool-1.0.0-windows-x64-portable.zip".to_owned(),
            os: Os::Windows,
            arch: Arch::X86_64,
            kind,
            sha256: Digest::of(b""),
            size: 1,
            launch: launch.map(str::to_owned),
            args: Vec::new(),
        }
    }

    #[test]
    fn installs_and_removes_a_portable_zip() {
        let root = scratch("zip");
        let package = root.join("package.zip");
        zip_with(&package, &[("bin/tool.exe", b"MZ"), ("readme.txt", b"hi")]);
        let target = Target {
            app: "tool",
            name: "Tool",
            version: "1.0.0",
            root: &root.join("apps"),
        };
        let installation =
            install(&asset(Kind::Zip, Some("bin/tool.exe")), &package, &target).unwrap();
        let launch = installation.launch.clone().unwrap();
        assert_eq!(fs::read(&launch).unwrap(), b"MZ");
        assert!(matches!(installation.removal, Some(Removal::Folder(_))));
        uninstall(&installation).unwrap();
        assert!(!launch.exists());
        assert!(!root.join("apps").join("tool").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_archives_that_escape_the_folder() {
        let root = scratch("escape");
        let package = root.join("evil.zip");
        zip_with(&package, &[("../../evil.exe", b"MZ")]);
        let target = Target {
            app: "evil",
            name: "Evil",
            version: "1.0.0",
            root: &root.join("apps"),
        };
        let result = install(&asset(Kind::Zip, Some("evil.exe")), &package, &target);
        assert!(matches!(result, Err(Error::Archive(_))), "{result:?}");
        assert!(!root.join("evil.exe").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn joins_only_paths_inside_the_root() {
        let root = Path::new("apps");
        assert_eq!(
            safe_join(root, Path::new("a/b.exe")),
            Some(root.join("a").join("b.exe"))
        );
        assert_eq!(safe_join(root, Path::new("../x")), None);
        assert_eq!(safe_join(root, Path::new("/etc/passwd")), None);
        assert_eq!(safe_join(root, Path::new(".")), None);
    }
}
