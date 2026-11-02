use crate::report::Finding;

const IDENTITY: &str = "T-G0W";
const NOREPLY: &str = "T-G0W@users.noreply.github.com";
const SUBJECT_LIMIT: usize = 72;
const PREFIXES: [&str; 11] = [
    "feat", "fix", "chore", "docs", "refactor", "test", "style", "perf", "build", "ci", "revert",
];
const TRAILERS: [&str; 2] = ["co-authored-by:", "signed-off-by:"];
const FIELD: char = '\u{1f}';
const RECORD: char = '\u{1e}';

pub(crate) const LOG_FORMAT: &str =
    "--format=%h%x1f%an%x1f%ae%x1f%ad%x1f%cn%x1f%ce%x1f%cd%x1f%B%x1e";

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Commit<'a> {
    hash: &'a str,
    author: &'a str,
    author_email: &'a str,
    author_date: &'a str,
    committer: &'a str,
    committer_email: &'a str,
    committer_date: &'a str,
    message: &'a str,
}

pub(crate) fn parse(log: &str) -> Vec<Commit<'_>> {
    log.split(RECORD)
        .filter_map(|record| {
            let mut fields = record.trim_start_matches('\n').splitn(8, FIELD);
            Some(Commit {
                hash: fields.next()?,
                author: fields.next()?,
                author_email: fields.next()?,
                author_date: fields.next()?,
                committer: fields.next()?,
                committer_email: fields.next()?,
                committer_date: fields.next()?,
                message: fields.next()?,
            })
        })
        .collect()
}

pub(crate) fn check_draft(message: &str, author: &str, committer: &str) -> Vec<Finding> {
    let message = message
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let (Some(author), Some(committer)) = (identity(author), identity(committer)) else {
        return vec![Finding::whole(
            "commit message",
            "git identity could not be read",
        )];
    };
    let draft = Commit {
        hash: "commit message",
        author: author.0,
        author_email: author.1,
        author_date: author.2,
        committer: committer.0,
        committer_email: committer.1,
        committer_date: committer.2,
        message: &message,
    };
    check(&[draft])
}

fn identity(ident: &str) -> Option<(&str, &str, &str)> {
    let (name, rest) = ident.trim().split_once(" <")?;
    let (email, date) = rest.split_once("> ")?;
    Some((name, email, date))
}

pub(crate) fn check(commits: &[Commit<'_>]) -> Vec<Finding> {
    commits
        .iter()
        .flat_map(|commit| {
            problems(commit)
                .into_iter()
                .map(|problem| Finding::whole(commit.hash, problem))
        })
        .collect()
}

fn problems(commit: &Commit<'_>) -> Vec<String> {
    let mut found = Vec::new();
    for (role, name, email, date) in [
        (
            "author",
            commit.author,
            commit.author_email,
            commit.author_date,
        ),
        (
            "committer",
            commit.committer,
            commit.committer_email,
            commit.committer_date,
        ),
    ] {
        if name != IDENTITY {
            found.push(format!("{role} name is not {IDENTITY}"));
        }
        if !(email == NOREPLY || email.ends_with(&format!("+{NOREPLY}"))) {
            found.push(format!("{role} email is not the github noreply address"));
        }
        if !date.ends_with(" +0000") {
            found.push(format!("{role} date is not in utc"));
        }
    }
    let subject = commit.message.lines().next().unwrap_or_default().trim_end();
    let length = subject.chars().count();
    if subject.is_empty() {
        found.push("subject is empty".to_owned());
    }
    if length > SUBJECT_LIMIT {
        found.push(format!(
            "subject has {length} characters, limit is {SUBJECT_LIMIT}"
        ));
    }
    if subject.ends_with('.') {
        found.push("subject ends with a period".to_owned());
    }
    if has_prefix(subject) {
        found.push("subject uses a conventional commit prefix".to_owned());
    }
    for line in commit.message.lines() {
        let lower = line.trim().to_ascii_lowercase();
        if TRAILERS.iter().any(|trailer| lower.starts_with(trailer)) {
            found.push(format!("trailer `{}`", line.trim()));
        }
        if lower.contains("generated with") {
            found.push("tool attribution in message".to_owned());
        }
    }
    if commit.message.chars().any(is_emoji) {
        found.push("emoji in message".to_owned());
    }
    found
}

fn has_prefix(subject: &str) -> bool {
    let Some((head, _)) = subject.split_once(':') else {
        return false;
    };
    let kind = head
        .trim_end_matches('!')
        .split('(')
        .next()
        .unwrap_or_default();
    PREFIXES.contains(&kind.to_ascii_lowercase().as_str())
}

fn is_emoji(c: char) -> bool {
    matches!(
        u32::from(c),
        0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0xFE0F | 0x200D
    )
}

#[cfg(test)]
mod draft_tests {
    use super::check_draft;

    const IDENT: &str = "T-G0W <210194347+T-G0W@users.noreply.github.com> 1730000000 +0000";

    #[test]
    fn accepts_a_clean_draft_and_ignores_comment_lines() {
        let message = "Add the reader\n\n# Please enter the commit message\n";
        assert_eq!(check_draft(message, IDENT, IDENT), Vec::new());
    }

    #[test]
    fn rejects_long_subjects_and_local_time_before_committing() {
        let long = format!("{}\n", "a".repeat(80));
        let local = "T-G0W <210194347+T-G0W@users.noreply.github.com> 1730000000 +0100";
        let findings: Vec<String> = check_draft(&long, local, IDENT)
            .into_iter()
            .map(|finding| finding.to_string())
            .collect();
        assert!(findings.iter().any(|text| text.contains("80 characters")));
        assert!(
            findings
                .iter()
                .any(|text| text.contains("author date is not in utc"))
        );
        assert_eq!(check_draft("Add the reader\n", "nobody", IDENT).len(), 1);
    }
}

#[cfg(test)]
mod tests {
    use super::{Commit, check, parse};

    fn commit(message: &str) -> Commit<'_> {
        Commit {
            hash: "abc1234",
            author: "T-G0W",
            author_email: "123+T-G0W@users.noreply.github.com",
            author_date: "1761955200 +0000",
            committer: "T-G0W",
            committer_email: "123+T-G0W@users.noreply.github.com",
            committer_date: "1761955200 +0000",
            message,
        }
    }

    #[test]
    fn accepts_plain_imperative_subjects() {
        let messages = [
            "Add EBML element reader\n",
            "Fix seek past the last cue point\n\nThe index ended one cluster early.\n",
        ];
        for message in messages {
            assert!(check(&[commit(message)]).is_empty(), "{message}");
        }
    }

    #[test]
    fn rejects_attribution_and_style_violations() {
        let cases = [
            "Add reader\n\nCo-Authored-By: Someone <a@b.c>\n",
            "Add reader\n\nGenerated with a tool\n",
            "feat: add reader\n",
            "fix(demux)!: handle cues\n",
            "Add reader.\n",
            "Add reader \u{1F916}\n",
            "\n",
            "Add a reader that is far too long for a subject line because it keeps going on\n",
        ];
        for message in cases {
            assert!(!check(&[commit(message)]).is_empty(), "{message:?}");
        }
    }

    #[test]
    fn rejects_foreign_identity_and_local_timezone() {
        let mut foreign = commit("Add reader\n");
        foreign.author = "Someone";
        foreign.committer_email = "someone@example.org";
        foreign.author_date = "1761955200 +0100";
        assert_eq!(check(&[foreign]).len(), 3);
    }

    #[test]
    fn parses_git_log_records() {
        let log = "abc\u{1f}T-G0W\u{1f}a@b\u{1f}1 +0000\u{1f}T-G0W\u{1f}a@b\u{1f}1 +0000\u{1f}Add a\n\u{1e}\ndef\u{1f}T-G0W\u{1f}a@b\u{1f}2 +0000\u{1f}T-G0W\u{1f}a@b\u{1f}2 +0000\u{1f}Add b\n\u{1e}\n";
        let commits = parse(log);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[1].hash, "def");
        assert_eq!(commits[1].message, "Add b\n");
    }
}
