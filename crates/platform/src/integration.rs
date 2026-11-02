use std::io;
use std::path::Path;

pub const HIDDEN_FLAG: &str = "--hidden";

pub fn set_autostart(enabled: bool, executable: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        let (run, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")?;
        if enabled {
            run.set_value(
                crate::paths::PRODUCT,
                &format!("\"{}\" {HIDDEN_FLAG}", executable.display()),
            )
        } else {
            match run.delete_value(crate::paths::PRODUCT) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            }
        }
    }
    #[cfg(not(windows))]
    {
        let Some(config) = std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".config")))
        else {
            return Ok(());
        };
        let file = config.join("autostart").join("tgow-launcher.desktop");
        if enabled {
            let entry = format!(
                "[Desktop Entry]\nType=Application\nName=T-GOW Launcher\nExec=\"{}\" {HIDDEN_FLAG}\nX-GNOME-Autostart-enabled=true\n",
                executable.display()
            );
            crate::storage::write_atomic(&file, entry.as_bytes())
        } else {
            match std::fs::remove_file(file) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            }
        }
    }
}

pub fn register_scheme(executable: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        use winreg::RegKey;
        use winreg::enums::HKEY_CURRENT_USER;
        let (scheme, _) =
            RegKey::predef(HKEY_CURRENT_USER).create_subkey(r"Software\Classes\tgow")?;
        scheme.set_value("", &"URL:T-GOW Launcher")?;
        scheme.set_value("URL Protocol", &"")?;
        let (command, _) = scheme.create_subkey(r"shell\open\command")?;
        command.set_value("", &format!("\"{}\" \"%1\"", executable.display()))
    }
    #[cfg(not(windows))]
    {
        let _ = executable;
        Ok(())
    }
}
