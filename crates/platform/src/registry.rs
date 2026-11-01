use std::path::PathBuf;

use tgow_core::asset::Kind;
use tgow_core::install::{Installation, Origin, Removal, split_command};
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
const UNINSTALL_32: &str = r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";

pub(crate) fn find(app: &str, name: &str) -> Option<Installation> {
    [
        (HKEY_CURRENT_USER, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL),
        (HKEY_LOCAL_MACHINE, UNINSTALL_32),
    ]
    .into_iter()
    .find_map(|(hive, path)| {
        let root = RegKey::predef(hive)
            .open_subkey_with_flags(path, KEY_READ)
            .ok()?;
        root.enum_keys().flatten().find_map(|key_name| {
            let key = root.open_subkey_with_flags(&key_name, KEY_READ).ok()?;
            let display: String = key.get_value("DisplayName").ok()?;
            let matches = display.eq_ignore_ascii_case(name) || display.eq_ignore_ascii_case(app);
            matches.then(|| entry(app, &key_name, &key))
        })
    })
}

fn entry(app: &str, key_name: &str, key: &RegKey) -> Installation {
    let text = |value: &str| {
        key.get_value::<String, _>(value)
            .ok()
            .filter(|text| !text.is_empty())
    };
    let windows_installer = key
        .get_value::<u32, _>("WindowsInstaller")
        .is_ok_and(|flag| flag == 1);
    let kind = if key_name.ends_with("_is1") {
        Kind::Inno
    } else if windows_installer {
        Kind::Msi
    } else {
        Kind::Nsis
    };
    let launch = text("DisplayIcon").map(|icon| {
        let icon = icon.trim_matches('"');
        let path = icon.rsplit_once(',').map_or(icon, |(path, index)| {
            if index.trim().parse::<i32>().is_ok() {
                path
            } else {
                icon
            }
        });
        PathBuf::from(path)
    });
    let removal = if kind == Kind::Msi {
        Some(Removal::Command {
            program: PathBuf::from("msiexec"),
            args: vec![format!("/X{key_name}")],
        })
    } else {
        text("QuietUninstallString")
            .or_else(|| text("UninstallString"))
            .and_then(|line| split_command(&line))
            .map(|(program, args)| Removal::Command { program, args })
    };
    Installation {
        app: app.to_owned(),
        version: text("DisplayVersion").unwrap_or_default(),
        kind,
        location: text("InstallLocation")
            .map(PathBuf::from)
            .or_else(|| {
                launch
                    .as_ref()
                    .and_then(|path| path.parent().map(PathBuf::from))
            })
            .unwrap_or_default(),
        launch: launch.filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
        }),
        removal,
        origin: Origin::Detected,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reading_the_uninstall_keys_never_fails() {
        assert!(super::find("tgow-launcher-test", "No Such App 7f3a").is_none());
    }
}
