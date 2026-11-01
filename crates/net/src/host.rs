pub const ALLOWED_HOSTS: [&str; 4] = [
    "api.github.com",
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

#[must_use]
pub(crate) fn name(url: &str) -> Option<&str> {
    let rest = url.strip_prefix("https://")?;
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..end];
    if host.is_empty() || host.contains(['@', ':', '\\']) {
        return None;
    }
    Some(host)
}

#[must_use]
pub fn is_allowed(url: &str) -> bool {
    name(url).is_some_and(|host| {
        ALLOWED_HOSTS
            .iter()
            .any(|allowed| host.eq_ignore_ascii_case(allowed))
    })
}

#[cfg(test)]
mod tests {
    use super::{is_allowed, name};

    #[test]
    fn allows_only_github_over_https() {
        for url in [
            "https://api.github.com/users/T-G0W/repos",
            "https://github.com/T-G0W/ferrovox/releases/latest",
            "https://release-assets.githubusercontent.com/github-production-release-asset/1?sig=x",
            "https://objects.githubusercontent.com/x",
        ] {
            assert!(is_allowed(url), "{url}");
        }
        for url in [
            "http://github.com/T-G0W",
            "https://github.com.evil.example/x",
            "https://evil.example/github.com",
            "https://user@github.com/x",
            "https://github.com:8443/x",
            "https://raw.githubusercontent.com/x",
            "ftp://github.com/x",
        ] {
            assert!(!is_allowed(url), "{url}");
        }
    }

    #[test]
    fn extracts_the_host_name() {
        assert_eq!(name("https://github.com/a?b"), Some("github.com"));
        assert_eq!(name("https://github.com"), Some("github.com"));
        assert_eq!(name("https:///x"), None);
    }
}
