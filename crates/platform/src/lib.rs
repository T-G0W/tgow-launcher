#![deny(unsafe_code)]

pub mod detect;
pub mod paths;
pub mod storage;

#[cfg(windows)]
#[expect(unsafe_code)]
mod windows;

#[must_use]
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}

#[must_use]
pub fn prefers_reduced_motion() -> bool {
    #[cfg(windows)]
    {
        !windows::client_animations_enabled()
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "enable-animations"])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "false")
    }
}
