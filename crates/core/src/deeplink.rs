use crate::manifest::is_app_id;

pub const SCHEME: &str = "tgow";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLink {
    Show,
    Open(String),
    Install(String),
}

impl DeepLink {
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().trim_matches('"');
        if text.is_empty() || text == "--show" {
            return Some(Self::Show);
        }
        let rest = text
            .strip_prefix(SCHEME)
            .and_then(|rest| rest.strip_prefix("://"))?;
        let rest = rest.trim_end_matches('/');
        if rest.is_empty() {
            return Some(Self::Show);
        }
        let (action, app) = rest.split_once('/')?;
        let app = app.to_ascii_lowercase();
        if !is_app_id(&app) {
            return None;
        }
        match action {
            "open" => Some(Self::Open(app)),
            "install" => Some(Self::Install(app)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DeepLink;

    #[test]
    fn parses_known_actions() {
        assert_eq!(
            DeepLink::parse("tgow://install/ferrovox"),
            Some(DeepLink::Install("ferrovox".to_owned()))
        );
        assert_eq!(
            DeepLink::parse("\"tgow://open/Okudro/\""),
            Some(DeepLink::Open("okudro".to_owned()))
        );
        assert_eq!(DeepLink::parse("tgow://"), Some(DeepLink::Show));
        assert_eq!(DeepLink::parse(""), Some(DeepLink::Show));
    }

    #[test]
    fn rejects_everything_else() {
        for text in [
            "tgow://run/ferrovox",
            "tgow://install/../../x",
            "tgow://install/a b",
            "https://install/ferrovox",
            "tgow://install/",
            "tgow:install/ferrovox",
        ] {
            assert_eq!(DeepLink::parse(text), None, "{text}");
        }
    }
}
