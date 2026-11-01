pub const GITHUB_PROFILE: &str = "https://github.com/T-G0W";
pub const REPOSITORY: &str = "https://github.com/T-G0W/tgow-launcher";
pub const WEBSITE: &str = "";
pub const DISCORD_COMMUNITY: &str = "";
pub const DISCORD_EDITING_HUB: &str = "";
pub const DISCORD_SERVICES: &str = "";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    DiscordCommunity,
    DiscordEditingHub,
    DiscordServices,
    Website,
    GitHub,
    Repository,
}

impl Destination {
    pub const ALL: [Self; 6] = [
        Self::DiscordCommunity,
        Self::DiscordEditingHub,
        Self::DiscordServices,
        Self::Website,
        Self::GitHub,
        Self::Repository,
    ];

    #[must_use]
    pub fn url(self) -> Option<&'static str> {
        let url = match self {
            Self::DiscordCommunity => DISCORD_COMMUNITY,
            Self::DiscordEditingHub => DISCORD_EDITING_HUB,
            Self::DiscordServices => DISCORD_SERVICES,
            Self::Website => WEBSITE,
            Self::GitHub => GITHUB_PROFILE,
            Self::Repository => REPOSITORY,
        };
        (!url.is_empty()).then_some(url)
    }

    #[must_use]
    pub fn is_discord(self) -> bool {
        matches!(
            self,
            Self::DiscordCommunity | Self::DiscordEditingHub | Self::DiscordServices
        )
    }

    #[must_use]
    pub fn main_discord() -> Option<&'static str> {
        [
            Self::DiscordCommunity,
            Self::DiscordEditingHub,
            Self::DiscordServices,
        ]
        .into_iter()
        .find_map(Self::url)
    }
}

#[cfg(test)]
mod tests {
    use super::Destination;

    #[test]
    fn every_known_url_is_https() {
        for url in Destination::ALL.into_iter().filter_map(Destination::url) {
            assert!(url.starts_with("https://"), "{url}");
        }
    }

    #[test]
    fn github_profile_uses_the_handle() {
        assert_eq!(Destination::GitHub.url(), Some("https://github.com/T-G0W"));
    }
}
