use serde::{Deserialize, Serialize};

use crate::checksum::Digest;
use crate::platform::{Arch, Os};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Exe,
    Zip,
    Inno,
    Nsis,
    Msi,
    #[serde(rename = "appimage")]
    AppImage,
    #[serde(rename = "targz")]
    TarGz,
    Deb,
    Rpm,
    Pacman,
    Flatpak,
    Dmg,
}

impl Kind {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Exe => "Portable",
            Self::Zip => "Portable ZIP",
            Self::Inno => "Inno Setup",
            Self::Nsis => "NSIS",
            Self::Msi => "MSI",
            Self::AppImage => "AppImage",
            Self::TarGz => "tar.gz",
            Self::Deb => "deb",
            Self::Rpm => "rpm",
            Self::Pacman => "pacman",
            Self::Flatpak => "Flatpak",
            Self::Dmg => "DMG",
        }
    }

    #[must_use]
    pub fn needs_root(self) -> bool {
        matches!(self, Self::Deb | Self::Rpm | Self::Pacman)
    }

    #[must_use]
    pub fn is_portable(self) -> bool {
        matches!(self, Self::Exe | Self::Zip | Self::AppImage | Self::TarGz)
    }

    #[must_use]
    pub fn is_installer(self) -> bool {
        matches!(self, Self::Inno | Self::Nsis | Self::Msi)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asset {
    pub file: String,
    pub os: Os,
    pub arch: Arch,
    pub kind: Kind,
    pub sha256: Digest,
    pub size: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub app: String,
    pub version: String,
    pub os: Os,
    pub arch: Arch,
    pub kind: Option<Kind>,
}

const EXTENSIONS: [(&str, Option<Kind>); 11] = [
    (".pkg.tar.zst", Some(Kind::Pacman)),
    (".tar.gz", Some(Kind::TarGz)),
    (".tgz", Some(Kind::TarGz)),
    (".appimage", Some(Kind::AppImage)),
    (".flatpak", Some(Kind::Flatpak)),
    (".deb", Some(Kind::Deb)),
    (".rpm", Some(Kind::Rpm)),
    (".msi", Some(Kind::Msi)),
    (".dmg", Some(Kind::Dmg)),
    (".zip", Some(Kind::Zip)),
    (".exe", None),
];

impl Name {
    #[must_use]
    pub fn parse(file: &str) -> Option<Self> {
        let lower = file.to_ascii_lowercase();
        let (extension, mut kind) = EXTENSIONS
            .iter()
            .find(|(extension, _)| lower.ends_with(extension))
            .copied()?;
        let stem = file.get(..file.len() - extension.len())?;
        let mut tokens: Vec<&str> = stem.split('-').collect();
        match tokens
            .last()
            .map(|last| last.to_ascii_lowercase())
            .as_deref()
        {
            Some("portable") => {
                tokens.pop();
                if extension == ".exe" {
                    kind = Some(Kind::Exe);
                }
            }
            Some("setup") => {
                tokens.pop();
            }
            _ => {}
        }
        let [app @ .., version, os, arch] = tokens.as_slice() else {
            return None;
        };
        if app.is_empty() || !version.starts_with(|c: char| c.is_ascii_digit()) {
            return None;
        }
        Some(Self {
            app: app.join("-").to_ascii_lowercase(),
            version: (*version).to_owned(),
            os: Os::parse(os)?,
            arch: Arch::parse(arch)?,
            kind,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, Name};
    use crate::platform::{Arch, Os};

    fn parsed(file: &str) -> Name {
        Name::parse(file).unwrap_or_else(|| panic!("{file}"))
    }

    #[test]
    fn parses_the_artcraft_release_names() {
        let cases = [
            (
                "artcraft-launcher-0.13.0-windows-x64.msi",
                Os::Windows,
                Arch::X86_64,
                Kind::Msi,
            ),
            (
                "artcraft-launcher-0.13.0-windows-arm64-portable.zip",
                Os::Windows,
                Arch::Aarch64,
                Kind::Zip,
            ),
            (
                "artcraft-launcher-0.13.0-linux-x86_64.AppImage",
                Os::Linux,
                Arch::X86_64,
                Kind::AppImage,
            ),
            (
                "artcraft-launcher-0.13.0-linux-aarch64.pkg.tar.zst",
                Os::Linux,
                Arch::Aarch64,
                Kind::Pacman,
            ),
            (
                "artcraft-launcher-0.13.0-linux-x86_64.tar.gz",
                Os::Linux,
                Arch::X86_64,
                Kind::TarGz,
            ),
            (
                "artcraft-launcher-0.13.0-macos-universal.dmg",
                Os::Macos,
                Arch::Universal,
                Kind::Dmg,
            ),
        ];
        for (file, os, arch, kind) in cases {
            let name = parsed(file);
            assert_eq!(name.app, "artcraft-launcher", "{file}");
            assert_eq!(name.version, "0.13.0", "{file}");
            assert_eq!(
                (name.os, name.arch, name.kind),
                (os, arch, Some(kind)),
                "{file}"
            );
        }
    }

    #[test]
    fn distinguishes_portable_exe_from_installer_exe() {
        assert_eq!(
            parsed("ferrovox-1.1.0-windows-x64-portable.exe").kind,
            Some(Kind::Exe)
        );
        assert_eq!(parsed("ferrovox-1.1.0-windows-x64.exe").kind, None);
        assert_eq!(parsed("ferrovox-1.1.0-windows-x64-setup.exe").kind, None);
    }

    #[test]
    fn ignores_names_outside_the_convention() {
        for file in [
            "Ferrovox-Setup-1.0.2.exe",
            "NeoCross_Setup.exe",
            "SHA256SUMS.txt",
            "artcraft-launcher-0.13.0-linux-x86_64.AppImage.zsync",
            "artcraft-launcher-web-0.13.0.zip",
            "artcraft-launcher-0.13.0-linux-riscv64.deb",
            "1.0.0-windows-x64.zip",
        ] {
            assert!(Name::parse(file).is_none(), "{file}");
        }
    }
}
