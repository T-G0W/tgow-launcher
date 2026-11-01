use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const SCHEMA: u32 = 1;
pub const CHECK_INTERVALS: [u32; 4] = [5, 15, 30, 60];
pub const PARALLEL_DOWNLOADS: std::ops::RangeInclusive<u8> = 1..=4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    German,
    #[default]
    English,
}

impl Language {
    #[must_use]
    pub fn from_locale(locale: &str) -> Self {
        if locale.to_ascii_lowercase().starts_with("de") {
            Self::German
        } else {
            Self::English
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageChoice {
    #[default]
    System,
    German,
    English,
}

impl LanguageChoice {
    #[must_use]
    pub fn resolve(self, system_locale: Option<&str>) -> Language {
        match self {
            Self::German => Language::German,
            Self::English => Language::English,
            Self::System => system_locale.map_or(Language::English, Language::from_locale),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Motion {
    #[default]
    System,
    Reduced,
    Full,
}

impl Motion {
    #[must_use]
    pub fn is_reduced(self, system_prefers_reduced: bool) -> bool {
        match self {
            Self::System => system_prefers_reduced,
            Self::Reduced => true,
            Self::Full => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateMode {
    #[default]
    Automatic,
    Notify,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    #[default]
    Stable,
    Beta,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub maximized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppPreference {
    pub updates: Option<UpdateMode>,
    pub channel: Channel,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[expect(clippy::struct_excessive_bools)]
pub struct Settings {
    pub schema: u32,
    pub language: LanguageChoice,
    pub motion: Motion,
    pub close_to_tray: bool,
    pub launch_at_login: bool,
    pub check_interval_minutes: u32,
    pub launcher_updates: UpdateMode,
    pub app_updates: UpdateMode,
    pub apps: BTreeMap<String, AppPreference>,
    pub parallel_downloads: u8,
    pub system_packages: bool,
    pub notifications: bool,
    pub ui_sounds: bool,
    pub sidebar_expanded: bool,
    pub install_root: Option<PathBuf>,
    pub window: Option<WindowGeometry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema: SCHEMA,
            language: LanguageChoice::System,
            motion: Motion::System,
            close_to_tray: true,
            launch_at_login: false,
            check_interval_minutes: 15,
            launcher_updates: UpdateMode::Automatic,
            app_updates: UpdateMode::Automatic,
            apps: BTreeMap::new(),
            parallel_downloads: 2,
            system_packages: false,
            notifications: true,
            ui_sounds: false,
            sidebar_expanded: false,
            install_root: None,
            window: None,
        }
    }
}

impl Settings {
    #[must_use]
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str::<Self>(text)
            .map(Self::normalized)
            .unwrap_or_default()
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    #[must_use]
    pub fn normalized(mut self) -> Self {
        self.schema = SCHEMA;
        if !CHECK_INTERVALS.contains(&self.check_interval_minutes) {
            self.check_interval_minutes = Self::default().check_interval_minutes;
        }
        self.parallel_downloads = self
            .parallel_downloads
            .clamp(*PARALLEL_DOWNLOADS.start(), *PARALLEL_DOWNLOADS.end());
        self.window = self.window.filter(|window| {
            window.width >= 640.0
                && window.height >= 400.0
                && window.x.is_finite()
                && window.y.is_finite()
        });
        self
    }

    #[must_use]
    pub fn update_mode(&self, app: &str) -> UpdateMode {
        self.apps
            .get(app)
            .and_then(|preference| preference.updates)
            .unwrap_or(self.app_updates)
    }

    #[must_use]
    pub fn channel(&self, app: &str) -> Channel {
        self.apps
            .get(app)
            .map_or(Channel::Stable, |preference| preference.channel)
    }
}

#[cfg(test)]
mod tests {
    use super::{Language, LanguageChoice, Motion, Settings, UpdateMode};

    #[test]
    fn defaults_update_everything_automatically() {
        let settings = Settings::default();
        assert_eq!(settings.launcher_updates, UpdateMode::Automatic);
        assert_eq!(settings.update_mode("okudro"), UpdateMode::Automatic);
        assert_eq!(settings.check_interval_minutes, 15);
    }

    #[test]
    fn broken_or_partial_files_fall_back_to_defaults() {
        assert_eq!(Settings::from_json("not json"), Settings::default());
        let partial = Settings::from_json(r#"{"close_to_tray":false}"#);
        assert!(!partial.close_to_tray);
        assert_eq!(partial.parallel_downloads, 2);
    }

    #[test]
    fn clamps_values_outside_the_allowed_range() {
        let settings = Settings::from_json(
            r#"{"check_interval_minutes":7,"parallel_downloads":40,"window":{"x":0,"y":0,"width":10,"height":10,"maximized":false}}"#,
        );
        assert_eq!(settings.check_interval_minutes, 15);
        assert_eq!(settings.parallel_downloads, 4);
        assert!(settings.window.is_none());
    }

    #[test]
    fn round_trips_through_json() {
        let settings = Settings {
            language: LanguageChoice::German,
            motion: Motion::Reduced,
            ..Settings::default()
        };
        assert_eq!(Settings::from_json(&settings.to_json()), settings);
    }

    #[test]
    fn resolves_language_from_the_system_locale() {
        assert_eq!(
            LanguageChoice::System.resolve(Some("de-DE")),
            Language::German
        );
        assert_eq!(
            LanguageChoice::System.resolve(Some("en-US")),
            Language::English
        );
        assert_eq!(LanguageChoice::System.resolve(None), Language::English);
        assert_eq!(
            LanguageChoice::German.resolve(Some("en-US")),
            Language::German
        );
    }

    #[test]
    fn motion_follows_the_system_only_when_asked() {
        assert!(Motion::System.is_reduced(true));
        assert!(!Motion::System.is_reduced(false));
        assert!(Motion::Reduced.is_reduced(false));
        assert!(!Motion::Full.is_reduced(true));
    }
}
