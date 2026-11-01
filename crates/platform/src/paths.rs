use std::env;
use std::path::{Path, PathBuf};

pub const PRODUCT: &str = "T-GOW Launcher";
pub const PORTABLE_MARKER: &str = "portable.txt";
const PORTABLE_DATA: &str = "T-GOW Launcher Data";
const LINUX_NAME: &str = "tgow-launcher";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
    pub apps: PathBuf,
    pub portable: bool,
}

impl Paths {
    #[must_use]
    pub fn locate() -> Option<Self> {
        let executable = env::current_exe().ok();
        let directory = executable.as_deref().and_then(Path::parent);
        if let Some(directory) =
            directory.filter(|directory| directory.join(PORTABLE_MARKER).is_file())
        {
            return Some(Self::portable(directory));
        }
        if cfg!(windows) {
            env::var_os("LOCALAPPDATA").map(|local| Self::windows(Path::new(&local)))
        } else {
            let home = env::var_os("HOME").map(PathBuf::from)?;
            Some(Self::xdg(&home, |name| {
                env::var_os(name).map(PathBuf::from)
            }))
        }
    }

    #[must_use]
    pub fn portable(directory: &Path) -> Self {
        let root = directory.join(PORTABLE_DATA);
        Self {
            config: root.clone(),
            cache: root.join("cache"),
            apps: root.join("apps"),
            data: root,
            portable: true,
        }
    }

    #[must_use]
    pub fn windows(local: &Path) -> Self {
        let data = local.join(PRODUCT);
        Self {
            config: data.clone(),
            cache: data.join("cache"),
            apps: local.join("Programs").join("T-GOW"),
            data,
            portable: false,
        }
    }

    #[must_use]
    pub fn xdg(home: &Path, variable: impl Fn(&str) -> Option<PathBuf>) -> Self {
        let base = |name: &str, fallback: &[&str]| {
            variable(name)
                .filter(|path| path.is_absolute())
                .unwrap_or_else(|| {
                    fallback
                        .iter()
                        .fold(home.to_path_buf(), |path, part| path.join(part))
                })
                .join(LINUX_NAME)
        };
        let data = base("XDG_DATA_HOME", &[".local", "share"]);
        Self {
            config: base("XDG_CONFIG_HOME", &[".config"]),
            cache: base("XDG_CACHE_HOME", &[".cache"]),
            apps: data.join("apps"),
            data,
            portable: false,
        }
    }

    #[must_use]
    pub fn settings(&self) -> PathBuf {
        self.config.join("settings.json")
    }

    #[must_use]
    pub fn installed(&self) -> PathBuf {
        self.data.join("installed.json")
    }

    #[must_use]
    pub fn downloads(&self) -> PathBuf {
        self.data.join("downloads")
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::Paths;

    #[test]
    fn windows_keeps_everything_under_local_app_data() {
        let paths = Paths::windows(Path::new("C:/Users/x/AppData/Local"));
        assert_eq!(
            paths.data,
            Path::new("C:/Users/x/AppData/Local/T-GOW Launcher")
        );
        assert_eq!(
            paths.apps,
            Path::new("C:/Users/x/AppData/Local/Programs/T-GOW")
        );
        assert_eq!(paths.settings(), paths.data.join("settings.json"));
    }

    #[test]
    fn xdg_ignores_relative_overrides() {
        let paths = Paths::xdg(Path::new("/home/x"), |_| Some(PathBuf::from("relative")));
        assert!(paths.cache.ends_with(".cache/tgow-launcher"));
        assert!(paths.data.ends_with(".local/share/tgow-launcher"));
    }

    #[cfg(unix)]
    #[test]
    fn xdg_honours_absolute_overrides() {
        let home = Path::new("/home/x");
        let paths = Paths::xdg(home, |name| match name {
            "XDG_CONFIG_HOME" => Some(PathBuf::from("/cfg")),
            "XDG_CACHE_HOME" => Some(PathBuf::from("relative")),
            _ => None,
        });
        assert_eq!(paths.config, Path::new("/cfg/tgow-launcher"));
        assert_eq!(paths.cache, Path::new("/home/x/.cache/tgow-launcher"));
        assert_eq!(paths.data, Path::new("/home/x/.local/share/tgow-launcher"));
    }

    #[test]
    fn portable_mode_stays_next_to_the_executable() {
        let paths = Paths::portable(Path::new("D:/stick"));
        assert!(paths.portable);
        assert!(paths.apps.starts_with("D:/stick/T-GOW Launcher Data"));
    }
}
