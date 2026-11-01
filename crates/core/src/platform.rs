use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Os {
    Windows,
    Linux,
    Macos,
}

impl Os {
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "windows" | "win" | "win64" => Some(Self::Windows),
            "linux" => Some(Self::Linux),
            "macos" | "darwin" | "osx" | "mac" => Some(Self::Macos),
            _ => None,
        }
    }

    #[must_use]
    pub fn token(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::Macos => "macos",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::Macos => "macOS",
        }
    }

    #[must_use]
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Arch {
    X86,
    X86_64,
    Aarch64,
    Universal,
}

impl Arch {
    #[must_use]
    pub fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "x86" | "i686" | "i386" => Some(Self::X86),
            "x64" | "x86_64" | "amd64" => Some(Self::X86_64),
            "arm64" | "aarch64" => Some(Self::Aarch64),
            "universal" => Some(Self::Universal),
            _ => None,
        }
    }

    #[must_use]
    pub fn token(self, os: Os) -> &'static str {
        match (self, os) {
            (Self::X86, _) => "x86",
            (Self::X86_64, Os::Windows) => "x64",
            (Self::X86_64, _) => "x86_64",
            (Self::Aarch64, Os::Windows) => "arm64",
            (Self::Aarch64, _) => "aarch64",
            (Self::Universal, _) => "universal",
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::X86 => "x86",
            Self::X86_64 => "x64",
            Self::Aarch64 => "ARM64",
            Self::Universal => "Universal",
        }
    }

    #[must_use]
    pub fn compiled() -> Self {
        if cfg!(target_arch = "aarch64") {
            Self::Aarch64
        } else if cfg!(target_arch = "x86") {
            Self::X86
        } else {
            Self::X86_64
        }
    }
}

impl Serialize for Os {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.token())
    }
}

impl<'de> Deserialize<'de> for Os {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let token = String::deserialize(deserializer)?;
        Self::parse(&token).ok_or_else(|| serde::de::Error::custom(format!("unknown os {token}")))
    }
}

impl Serialize for Arch {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.token(Os::Linux))
    }
}

impl<'de> Deserialize<'de> for Arch {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let token = String::deserialize(deserializer)?;
        Self::parse(&token)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown architecture {token}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Debian,
    Fedora,
    Suse,
    Arch,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Distro {
    pub id: String,
    pub like: Vec<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

impl Distro {
    #[must_use]
    pub fn parse_os_release(text: &str) -> Option<Self> {
        let mut distro = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.trim().split_once('=') else {
                continue;
            };
            let value = value.trim().trim_matches(['"', '\'']);
            match key {
                "ID" => distro.id = value.to_ascii_lowercase(),
                "ID_LIKE" => {
                    distro.like = value
                        .split_whitespace()
                        .map(str::to_ascii_lowercase)
                        .collect();
                }
                "PRETTY_NAME" => distro.name = Some(value.to_owned()),
                "VERSION_ID" => distro.version = Some(value.to_owned()),
                _ => {}
            }
        }
        (!distro.id.is_empty()).then_some(distro)
    }

    #[must_use]
    pub fn family(&self) -> Family {
        std::iter::once(self.id.as_str())
            .chain(self.like.iter().map(String::as_str))
            .find_map(family_of)
            .unwrap_or(Family::Other)
    }
}

fn family_of(id: &str) -> Option<Family> {
    match id {
        "debian" | "ubuntu" | "linuxmint" | "pop" | "elementary" | "zorin" | "kali" => {
            Some(Family::Debian)
        }
        "fedora" | "rhel" | "centos" | "rocky" | "almalinux" | "nobara" => Some(Family::Fedora),
        "suse" | "sles" | "opensuse" | "opensuse-leap" | "opensuse-tumbleweed" => {
            Some(Family::Suse)
        }
        "arch" | "manjaro" | "endeavouros" | "garuda" | "cachyos" | "artix" => Some(Family::Arch),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Libc {
    Gnu,
    Musl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Sandbox {
    #[default]
    Off,
    Flatpak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
    pub build: Option<u32>,
    pub version: Option<String>,
    pub distro: Option<Distro>,
    pub libc: Option<Libc>,
    pub sandbox: Sandbox,
    pub host_flatpak: bool,
    pub immutable: bool,
    pub pkexec: bool,
}

impl Platform {
    #[must_use]
    pub fn new(os: Os, arch: Arch) -> Self {
        Self {
            os,
            arch,
            build: None,
            version: None,
            distro: None,
            libc: None,
            sandbox: Sandbox::Off,
            host_flatpak: false,
            immutable: false,
            pkexec: false,
        }
    }

    #[must_use]
    pub fn compatible_arches(&self) -> Vec<Arch> {
        match (self.os, self.arch) {
            (Os::Windows, Arch::Aarch64) if self.build.is_some_and(|build| build >= 22_000) => {
                vec![Arch::Aarch64, Arch::X86_64, Arch::X86]
            }
            (Os::Windows, Arch::Aarch64) => vec![Arch::Aarch64, Arch::X86],
            (Os::Windows, Arch::X86_64) => vec![Arch::X86_64, Arch::X86],
            (Os::Macos, Arch::Aarch64) => vec![Arch::Universal, Arch::Aarch64, Arch::X86_64],
            (Os::Macos, _) => vec![Arch::Universal, Arch::X86_64],
            (_, arch) => vec![arch],
        }
    }

    #[must_use]
    pub fn is_emulated(&self, arch: Arch) -> bool {
        let runs_natively = arch == self.arch
            || arch == Arch::Universal
            || (arch == Arch::X86 && self.arch == Arch::X86_64 && self.os == Os::Windows);
        !runs_natively
    }

    #[must_use]
    pub fn family(&self) -> Family {
        self.distro.as_ref().map_or(Family::Other, Distro::family)
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.distro, &self.version) {
            (
                Some(Distro {
                    name: Some(name), ..
                }),
                _,
            ) => write!(f, "{name}")?,
            (_, Some(version)) => write!(f, "{} {version}", self.os.label())?,
            _ => f.write_str(self.os.label())?,
        }
        write!(f, " · {}", self.arch.label())
    }
}

#[cfg(test)]
mod tests {
    use super::{Arch, Distro, Family, Os, Platform};

    #[test]
    fn parses_architecture_aliases() {
        let cases = [
            ("x64", Some(Arch::X86_64)),
            ("AMD64", Some(Arch::X86_64)),
            ("x86_64", Some(Arch::X86_64)),
            ("arm64", Some(Arch::Aarch64)),
            ("aarch64", Some(Arch::Aarch64)),
            ("i686", Some(Arch::X86)),
            ("universal", Some(Arch::Universal)),
            ("riscv64", None),
        ];
        for (token, expected) in cases {
            assert_eq!(Arch::parse(token), expected, "{token}");
        }
    }

    #[test]
    fn names_arches_like_the_asset_convention() {
        assert_eq!(Arch::X86_64.token(Os::Windows), "x64");
        assert_eq!(Arch::X86_64.token(Os::Linux), "x86_64");
        assert_eq!(Arch::Aarch64.token(Os::Windows), "arm64");
        assert_eq!(Arch::Aarch64.token(Os::Linux), "aarch64");
    }

    #[test]
    fn windows_arm_runs_x64_only_from_windows_11() {
        let mut platform = Platform::new(Os::Windows, Arch::Aarch64);
        platform.build = Some(19_045);
        assert_eq!(platform.compatible_arches(), vec![Arch::Aarch64, Arch::X86]);
        platform.build = Some(26_200);
        assert_eq!(
            platform.compatible_arches(),
            vec![Arch::Aarch64, Arch::X86_64, Arch::X86]
        );
    }

    #[test]
    fn x86_on_x64_windows_counts_as_native() {
        let platform = Platform::new(Os::Windows, Arch::X86_64);
        assert!(!platform.is_emulated(Arch::X86));
        assert!(!platform.is_emulated(Arch::X86_64));
        let arm = Platform::new(Os::Windows, Arch::Aarch64);
        assert!(arm.is_emulated(Arch::X86_64));
    }

    #[test]
    fn reads_os_release_and_family() {
        let text = "NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nVERSION_ID=\"24.04\"\n";
        let distro = Distro::parse_os_release(text).unwrap();
        assert_eq!(distro.id, "ubuntu");
        assert_eq!(distro.like, vec!["debian".to_owned()]);
        assert_eq!(distro.version.as_deref(), Some("24.04"));
        assert_eq!(distro.family(), Family::Debian);
    }

    #[test]
    fn falls_back_to_id_like_for_derivatives() {
        let distro = Distro::parse_os_release("ID=bazzite\nID_LIKE=\"fedora\"\n").unwrap();
        assert_eq!(distro.family(), Family::Fedora);
        let unknown = Distro::parse_os_release("ID=nixos\n").unwrap();
        assert_eq!(unknown.family(), Family::Other);
        assert!(Distro::parse_os_release("NAME=x\n").is_none());
    }

    #[test]
    fn displays_distro_name_before_os_version() {
        let mut platform = Platform::new(Os::Windows, Arch::X86_64);
        platform.version = Some("11".to_owned());
        assert_eq!(platform.to_string(), "Windows 11 · x64");
    }
}
