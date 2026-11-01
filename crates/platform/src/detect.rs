use std::env;
use std::fs;
use std::path::Path;

use tgow_core::platform::{Arch, Distro, Libc, Os, Platform, Sandbox};

#[must_use]
pub fn platform() -> Platform {
    match Os::current() {
        Os::Windows => windows(),
        Os::Linux => linux(),
        Os::Macos => Platform::new(Os::Macos, Arch::compiled()),
    }
}

#[cfg(windows)]
fn windows() -> Platform {
    let mut platform = Platform::new(
        Os::Windows,
        crate::windows::native_arch().unwrap_or_else(Arch::compiled),
    );
    platform.build = crate::windows::build_number();
    platform.version = platform.build.map(|build| {
        let generation = if build >= 22_000 { "11" } else { "10" };
        format!("{generation} ({build})")
    });
    platform
}

#[cfg(not(windows))]
fn windows() -> Platform {
    Platform::new(Os::Windows, Arch::compiled())
}

fn linux() -> Platform {
    let mut platform = Platform::new(Os::Linux, Arch::compiled());
    platform.distro = ["/etc/os-release", "/usr/lib/os-release"]
        .into_iter()
        .find_map(|path| fs::read_to_string(path).ok())
        .and_then(|text| Distro::parse_os_release(&text));
    platform.libc = Some(if has_musl_loader() {
        Libc::Musl
    } else {
        Libc::Gnu
    });
    if Path::new("/.flatpak-info").exists() {
        platform.sandbox = Sandbox::Flatpak;
    }
    platform.host_flatpak = on_path("flatpak");
    platform.immutable = Path::new("/run/ostree-booted").exists();
    platform.pkexec = on_path("pkexec");
    platform
}

fn has_musl_loader() -> bool {
    fs::read_dir("/lib").is_ok_and(|entries| {
        entries
            .flatten()
            .any(|entry| entry.file_name().to_string_lossy().starts_with("ld-musl-"))
    })
}

fn on_path(program: &str) -> bool {
    env::var_os("PATH").is_some_and(|path| {
        env::split_paths(&path).any(|directory| directory.join(program).is_file())
    })
}

#[cfg(test)]
mod tests {
    use tgow_core::platform::Os;

    #[test]
    fn detects_the_running_system() {
        let platform = super::platform();
        assert_eq!(platform.os, Os::current());
        assert!(platform.compatible_arches().contains(&platform.arch));
        if platform.os == Os::Windows {
            assert!(platform.build.is_some_and(|build| build >= 10_240));
        }
    }
}
