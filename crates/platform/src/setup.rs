use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[cfg(windows)]
use crate::paths::PRODUCT;
use crate::paths::Paths;

pub const UNINSTALL_FLAG: &str = "--uninstall-self";
pub const EXECUTABLE: &str = if cfg!(windows) {
    "tgow-launcher.exe"
} else {
    "tgow-launcher"
};

#[cfg(windows)]
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\TGOW.Launcher";

#[must_use]
pub fn needs_install(paths: &Paths, executable: &Path) -> bool {
    cfg!(windows)
        && !cfg!(debug_assertions)
        && !paths.portable
        && !paths.is_installed_copy(executable)
}

pub fn install(paths: &Paths, executable: &Path, version: &str) -> io::Result<PathBuf> {
    let home = paths.home();
    fs::create_dir_all(&home)?;
    let target = home.join(EXECUTABLE);
    fs::copy(executable, &target)?;
    register(&target, version)?;
    crate::integration::register_scheme(&target)?;
    Ok(target)
}

pub fn uninstall(paths: &Paths) -> io::Result<()> {
    let target = paths.home().join(EXECUTABLE);
    let _ = crate::integration::set_autostart(false, &target);
    unregister()?;
    self_replace::self_delete()
}

#[cfg(windows)]
fn start_menu() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|roaming| {
        Path::new(&roaming)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join(format!("{PRODUCT}.lnk"))
    })
}

#[cfg(windows)]
fn register(target: &Path, version: &str) -> io::Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    if let Some(link) = start_menu() {
        let mut shortcut = mslnk::ShellLink::new(target).map_err(io::Error::other)?;
        shortcut.set_name(Some(PRODUCT.to_owned()));
        shortcut.set_working_dir(target.parent().map(|folder| folder.display().to_string()));
        shortcut.set_icon_location(Some(target.display().to_string()));
        shortcut.create_lnk(link).map_err(io::Error::other)?;
    }
    let size = fs::metadata(target).map_or(0, |metadata| metadata.len() / 1024);
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(UNINSTALL_KEY)?;
    let program = target.display().to_string();
    key.set_value("DisplayName", &PRODUCT)?;
    key.set_value("DisplayVersion", &version)?;
    key.set_value("Publisher", &"T-GOW")?;
    key.set_value("DisplayIcon", &program)?;
    key.set_value(
        "InstallLocation",
        &target
            .parent()
            .map(|folder| folder.display().to_string())
            .unwrap_or_default(),
    )?;
    key.set_value(
        "UninstallString",
        &format!("\"{program}\" {UNINSTALL_FLAG}"),
    )?;
    key.set_value(
        "QuietUninstallString",
        &format!("\"{program}\" {UNINSTALL_FLAG}"),
    )?;
    key.set_value("NoModify", &1_u32)?;
    key.set_value("NoRepair", &1_u32)?;
    key.set_value("EstimatedSize", &u32::try_from(size).unwrap_or(u32::MAX))?;
    Ok(())
}

#[cfg(windows)]
fn unregister() -> io::Result<()> {
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;
    if let Some(link) = start_menu() {
        let _ = fs::remove_file(link);
    }
    let user = RegKey::predef(HKEY_CURRENT_USER);
    for key in [UNINSTALL_KEY, r"Software\Classes\tgow"] {
        match user.delete_subkey_all(key) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn desktop_entry() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME").map(|home| Path::new(&home).join(".local").join("share"))
        })
        .map(|data| data.join("applications").join("tgow-launcher.desktop"))
}

#[cfg(not(windows))]
fn register(target: &Path, version: &str) -> io::Result<()> {
    let Some(file) = desktop_entry() else {
        return Ok(());
    };
    let entry = format!(
        "[Desktop Entry]
Type=Application
Name=T-GOW Launcher
Version=1.5
X-TGOW-Version={version}
Exec=\"{}\" %u
Icon=io.github.T_G0W.Launcher
Categories=Utility;
MimeType=x-scheme-handler/tgow;
StartupWMClass=io.github.T_G0W.Launcher
",
        target.display()
    );
    crate::storage::write_atomic(&file, entry.as_bytes())
}

#[cfg(not(windows))]
fn unregister() -> io::Result<()> {
    match desktop_entry().map(fs::remove_file) {
        Some(Err(error)) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}
