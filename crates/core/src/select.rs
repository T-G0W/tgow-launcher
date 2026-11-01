use crate::asset::{Asset, Kind};
use crate::platform::{Arch, Family, Os, Platform, Sandbox};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Preference {
    pub installed: Option<Kind>,
    pub chosen: Option<Kind>,
    pub system_packages: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    KeepsInstalledKind,
    ChosenByUser,
    BestForPlatform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Choice<'a> {
    pub asset: &'a Asset,
    pub reason: Reason,
    pub emulated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rejection {
    NoAssets,
    Unsupported { offered: Vec<(Os, Arch)> },
}

#[must_use]
pub fn kind_order(platform: &Platform, system_packages: bool) -> Vec<Kind> {
    match platform.os {
        Os::Windows => vec![Kind::Exe, Kind::Zip, Kind::Msi, Kind::Inno, Kind::Nsis],
        Os::Macos => vec![Kind::Dmg],
        Os::Linux => linux_order(platform, system_packages),
    }
}

fn linux_order(platform: &Platform, system_packages: bool) -> Vec<Kind> {
    if platform.sandbox == Sandbox::Flatpak {
        return if platform.host_flatpak {
            vec![Kind::Flatpak, Kind::TarGz]
        } else {
            vec![Kind::TarGz]
        };
    }
    if platform.immutable {
        return vec![Kind::Flatpak, Kind::AppImage, Kind::TarGz];
    }
    let mut order = vec![Kind::AppImage, Kind::TarGz];
    if platform.host_flatpak {
        order.push(Kind::Flatpak);
    }
    let native = match platform.family() {
        Family::Debian => Some(Kind::Deb),
        Family::Fedora | Family::Suse => Some(Kind::Rpm),
        Family::Arch => Some(Kind::Pacman),
        Family::Other => None,
    };
    if let Some(native) = native.filter(|_| platform.pkexec) {
        if system_packages {
            order.insert(0, native);
        } else {
            order.push(native);
        }
    }
    order
}

pub fn choose<'a>(
    assets: &'a [Asset],
    platform: &Platform,
    preference: Preference,
) -> Result<Choice<'a>, Rejection> {
    if assets.is_empty() {
        return Err(Rejection::NoAssets);
    }
    let arches = platform.compatible_arches();
    let order = kind_order(platform, preference.system_packages);
    let pinned = [
        (preference.installed, Reason::KeepsInstalledKind),
        (preference.chosen, Reason::ChosenByUser),
    ];
    for (kind, reason) in pinned {
        let Some(kind) = kind.filter(|kind| order.contains(kind)) else {
            continue;
        };
        if let Some(choice) = find(assets, platform, &arches, &[kind], reason) {
            return Ok(choice);
        }
    }
    find(assets, platform, &arches, &order, Reason::BestForPlatform).ok_or_else(|| {
        let mut offered: Vec<(Os, Arch)> =
            assets.iter().map(|asset| (asset.os, asset.arch)).collect();
        offered.dedup();
        Rejection::Unsupported { offered }
    })
}

fn find<'a>(
    assets: &'a [Asset],
    platform: &Platform,
    arches: &[Arch],
    kinds: &[Kind],
    reason: Reason,
) -> Option<Choice<'a>> {
    arches.iter().find_map(|&arch| {
        kinds.iter().find_map(|&kind| {
            assets
                .iter()
                .find(|asset| asset.os == platform.os && asset.arch == arch && asset.kind == kind)
                .map(|asset| Choice {
                    asset,
                    reason,
                    emulated: platform.is_emulated(arch),
                })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{Preference, Reason, Rejection, choose, kind_order};
    use crate::asset::{Asset, Kind, Name};
    use crate::checksum::Digest;
    use crate::platform::{Arch, Distro, Os, Platform, Sandbox};

    const ARTCRAFT: [&str; 20] = [
        "artcraft-launcher-0.13.0-linux-aarch64.AppImage",
        "artcraft-launcher-0.13.0-linux-aarch64.deb",
        "artcraft-launcher-0.13.0-linux-aarch64.flatpak",
        "artcraft-launcher-0.13.0-linux-aarch64.pkg.tar.zst",
        "artcraft-launcher-0.13.0-linux-aarch64.rpm",
        "artcraft-launcher-0.13.0-linux-aarch64.tar.gz",
        "artcraft-launcher-0.13.0-linux-x86_64.AppImage",
        "artcraft-launcher-0.13.0-linux-x86_64.deb",
        "artcraft-launcher-0.13.0-linux-x86_64.flatpak",
        "artcraft-launcher-0.13.0-linux-x86_64.pkg.tar.zst",
        "artcraft-launcher-0.13.0-linux-x86_64.rpm",
        "artcraft-launcher-0.13.0-linux-x86_64.tar.gz",
        "artcraft-launcher-0.13.0-macos-universal.dmg",
        "artcraft-launcher-0.13.0-windows-arm64-portable.zip",
        "artcraft-launcher-0.13.0-windows-arm64.msi",
        "artcraft-launcher-0.13.0-windows-x64-portable.zip",
        "artcraft-launcher-0.13.0-windows-x64.msi",
        "artcraft-launcher-0.13.0-windows-x86-portable.zip",
        "artcraft-launcher-0.13.0-windows-x86.msi",
        "artcraft-launcher-0.13.0-linux-riscv64.tar.gz",
    ];

    fn assets(files: &[&str]) -> Vec<Asset> {
        files
            .iter()
            .filter_map(|file| {
                let name = Name::parse(file)?;
                Some(Asset {
                    file: (*file).to_owned(),
                    os: name.os,
                    arch: name.arch,
                    kind: name.kind?,
                    sha256: Digest::of(file.as_bytes()),
                    size: 1,
                    launch: None,
                    args: Vec::new(),
                })
            })
            .collect()
    }

    fn linux(id: &str, like: &str) -> Platform {
        let mut platform = Platform::new(Os::Linux, Arch::X86_64);
        platform.distro = Distro::parse_os_release(&format!("ID={id}\nID_LIKE={like}\n"));
        platform.pkexec = true;
        platform
    }

    fn chosen(platform: &Platform, preference: Preference) -> String {
        let all = assets(&ARTCRAFT);
        choose(&all, platform, preference)
            .unwrap()
            .asset
            .file
            .clone()
    }

    #[test]
    fn picks_the_best_artcraft_asset_per_platform() {
        let mut windows11_arm = Platform::new(Os::Windows, Arch::Aarch64);
        windows11_arm.build = Some(26_200);
        let mut immutable = linux("bazzite", "fedora");
        immutable.immutable = true;
        immutable.host_flatpak = true;
        let cases = [
            (
                Platform::new(Os::Windows, Arch::X86_64),
                "windows-x64-portable.zip",
            ),
            (windows11_arm, "windows-arm64-portable.zip"),
            (
                Platform::new(Os::Windows, Arch::X86),
                "windows-x86-portable.zip",
            ),
            (linux("ubuntu", "debian"), "linux-x86_64.AppImage"),
            (immutable, "linux-x86_64.flatpak"),
            (
                Platform::new(Os::Macos, Arch::Aarch64),
                "macos-universal.dmg",
            ),
        ];
        for (platform, expected) in cases {
            let file = chosen(&platform, Preference::default());
            assert!(file.ends_with(expected), "{platform}: {file}");
        }
    }

    #[test]
    fn system_packages_follow_the_distro_family() {
        let preference = Preference {
            system_packages: true,
            ..Preference::default()
        };
        for (id, like, expected) in [
            ("ubuntu", "debian", ".deb"),
            ("fedora", "", ".rpm"),
            ("opensuse-tumbleweed", "suse", ".rpm"),
            ("cachyos", "arch", ".pkg.tar.zst"),
        ] {
            let file = chosen(&linux(id, like), preference);
            assert!(file.ends_with(expected), "{id}: {file}");
        }
        let mut without_pkexec = linux("ubuntu", "debian");
        without_pkexec.pkexec = false;
        assert!(chosen(&without_pkexec, preference).ends_with(".AppImage"));
    }

    #[test]
    fn keeps_the_installed_kind_before_anything_else() {
        let preference = Preference {
            installed: Some(Kind::Msi),
            chosen: Some(Kind::Zip),
            system_packages: false,
        };
        let all = assets(&ARTCRAFT);
        let choice = choose(&all, &Platform::new(Os::Windows, Arch::X86_64), preference).unwrap();
        assert_eq!(
            choice.asset.file,
            "artcraft-launcher-0.13.0-windows-x64.msi"
        );
        assert_eq!(choice.reason, Reason::KeepsInstalledKind);
    }

    #[test]
    fn falls_back_to_emulation_and_says_so() {
        let x64_only = assets(&["tool-1.0.0-windows-x64-portable.zip"]);
        let mut arm = Platform::new(Os::Windows, Arch::Aarch64);
        arm.build = Some(26_200);
        let choice = choose(&x64_only, &arm, Preference::default()).unwrap();
        assert!(choice.emulated);
        arm.build = Some(19_045);
        assert!(choose(&x64_only, &arm, Preference::default()).is_err());
    }

    #[test]
    fn explains_what_is_offered_when_nothing_fits() {
        let linux_only = assets(&["tool-1.0.0-linux-x86_64.tar.gz"]);
        let rejection = choose(
            &linux_only,
            &Platform::new(Os::Windows, Arch::X86_64),
            Preference::default(),
        );
        assert_eq!(
            rejection,
            Err(Rejection::Unsupported {
                offered: vec![(Os::Linux, Arch::X86_64)]
            })
        );
        assert_eq!(
            choose(
                &[],
                &Platform::new(Os::Linux, Arch::X86_64),
                Preference::default()
            ),
            Err(Rejection::NoAssets)
        );
    }

    #[test]
    fn inside_flatpak_never_offers_appimages() {
        let mut sandboxed = linux("ubuntu", "debian");
        sandboxed.sandbox = Sandbox::Flatpak;
        assert_eq!(kind_order(&sandboxed, true), vec![Kind::TarGz]);
    }
}
