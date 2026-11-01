use std::time::Duration;

const LONGEST_WAIT: Duration = Duration::from_secs(3600);

#[must_use]
pub fn retry_after(
    retry_after: Option<&str>,
    remaining: Option<&str>,
    reset: Option<&str>,
    now: u64,
) -> Option<Duration> {
    let explicit = retry_after.and_then(|value| value.trim().parse::<u64>().ok());
    let exhausted = remaining.is_some_and(|value| value.trim() == "0");
    let until_reset = reset
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|_| exhausted)
        .map(|reset| reset.saturating_sub(now).max(1));
    explicit
        .or(until_reset)
        .map(|seconds| Duration::from_secs(seconds).min(LONGEST_WAIT))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::retry_after;

    #[test]
    fn prefers_the_explicit_retry_after_header() {
        assert_eq!(
            retry_after(Some("30"), Some("0"), Some("2000"), 1000),
            Some(Duration::from_secs(30))
        );
    }

    #[test]
    fn waits_until_reset_only_when_the_quota_is_used_up() {
        assert_eq!(
            retry_after(None, Some("0"), Some("1600"), 1000),
            Some(Duration::from_secs(600))
        );
        assert_eq!(retry_after(None, Some("12"), Some("1600"), 1000), None);
        assert_eq!(retry_after(None, None, None, 1000), None);
    }

    #[test]
    fn never_waits_longer_than_an_hour() {
        assert_eq!(
            retry_after(None, Some("0"), Some("99999"), 1000),
            Some(Duration::from_secs(3600))
        );
    }
}
