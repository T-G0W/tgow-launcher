use std::fs;
use std::path::Path;

use crate::error::Error;
use crate::report::Finding;

const MIN_RUN: usize = 6;
const SNIPPET: usize = 80;

const HOME_MARKERS: [&str; 5] = [
    ":\\users\\",
    ":/users/",
    ":\\\\users\\\\",
    "/home/",
    "\\home\\",
];
const MAC_HOME: &str = "/Users/";
const THIRD_PARTY_EMAILS: [&str; 1] = ["appro@openssl.org"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct Run {
    offset: usize,
    text: String,
}

pub(crate) fn check(path: &Path) -> Result<Vec<Finding>, Error> {
    let bytes = fs::read(path).map_err(|error| Error::Read(path.to_path_buf(), error))?;
    let name = path.display().to_string();
    let mut runs = ascii_runs(&bytes);
    runs.extend(utf16_runs(&bytes));
    Ok(runs
        .iter()
        .flat_map(|run| inspect(run))
        .map(|(offset, kind, snippet)| {
            Finding::whole(&name, format!("{kind} at {offset:#x}: {snippet}"))
        })
        .collect())
}

fn inspect(run: &Run) -> Vec<(usize, &'static str, String)> {
    let mut found = Vec::new();
    let lower = run.text.to_ascii_lowercase();
    let home = HOME_MARKERS
        .iter()
        .filter_map(|marker| lower.find(marker))
        .chain(run.text.find(MAC_HOME))
        .min();
    if let Some(index) = home {
        found.push((
            run.offset + index,
            "home directory path",
            snippet(&run.text, index),
        ));
    }
    for index in emails(&run.text) {
        found.push((
            run.offset + index,
            "email address",
            snippet(&run.text, index),
        ));
    }
    found
}

fn snippet(text: &str, index: usize) -> String {
    let start = index.saturating_sub(SNIPPET / 4);
    text.get(start..)
        .unwrap_or(text)
        .chars()
        .take(SNIPPET)
        .collect()
}

fn emails(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let local = |byte: u8| byte.is_ascii_alphanumeric() || b"._%+-".contains(&byte);
    let domain = |byte: u8| byte.is_ascii_alphanumeric() || b".-".contains(&byte);
    let mut found = Vec::new();
    for (at, _) in text.match_indices('@') {
        let start = bytes[..at]
            .iter()
            .rposition(|byte| !local(*byte))
            .map_or(0, |index| index + 1);
        let end = bytes[at + 1..]
            .iter()
            .position(|byte| !domain(*byte))
            .map_or(bytes.len(), |index| at + 1 + index);
        let host = text[at + 1..end].trim_end_matches(['.', '-']);
        let tld = host.rsplit('.').next().unwrap_or_default();
        let plausible = text[start..at]
            .bytes()
            .any(|byte| byte.is_ascii_alphanumeric())
            && host.contains('.')
            && host.split('.').all(|label| !label.is_empty())
            && tld.len() >= 2
            && tld.bytes().all(|byte| byte.is_ascii_alphabetic());
        let address = &text[start..end];
        if plausible
            && !THIRD_PARTY_EMAILS
                .iter()
                .any(|known| address.starts_with(known))
        {
            found.push(start);
        }
    }
    found
}

fn ascii_runs(bytes: &[u8]) -> Vec<Run> {
    let mut runs = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().chain([&0]).enumerate() {
        if is_text(*byte) {
            continue;
        }
        if index - start >= MIN_RUN {
            let text = String::from_utf8_lossy(&bytes[start..index]).into_owned();
            runs.push(Run {
                offset: start,
                text,
            });
        }
        start = index + 1;
    }
    runs
}

fn utf16_runs(bytes: &[u8]) -> Vec<Run> {
    let mut runs = Vec::new();
    for parity in 0..2 {
        let mut text = String::new();
        let mut start = parity;
        let mut index = parity;
        while index <= bytes.len() {
            let pair = bytes.get(index).zip(bytes.get(index + 1));
            match pair {
                Some((low, 0)) if is_text(*low) => text.push(char::from(*low)),
                _ => {
                    if text.len() >= MIN_RUN {
                        runs.push(Run {
                            offset: start,
                            text: std::mem::take(&mut text),
                        });
                    }
                    start = index + 2;
                }
            }
            index += 2;
        }
    }
    runs
}

fn is_text(byte: u8) -> bool {
    byte.is_ascii_graphic() || byte == b' '
}

#[cfg(test)]
mod tests {
    use super::{Run, ascii_runs, emails, inspect, utf16_runs};

    fn run(text: &str) -> Run {
        Run {
            offset: 0,
            text: text.to_owned(),
        }
    }

    #[test]
    fn finds_home_paths_but_not_api_urls() {
        let cases = [
            ("C:\\Users\\someone\\.cargo\\registry\\src", true),
            ("c:/users/someone/project/src/main.rs", true),
            ("/home/someone/.cargo/registry/src/x.rs", true),
            ("/Users/someone/dev/app/src/lib.rs", true),
            ("https://api.github.com/users/T-G0W/repos", false),
            ("/rustc/abc/library/std/src/panicking.rs", false),
            ("/src/crates/core/src/manifest.rs", false),
        ];
        for (text, expected) in cases {
            let hits = inspect(&run(text));
            assert_eq!(
                hits.iter().any(|hit| hit.1 == "home directory path"),
                expected,
                "{text}"
            );
        }
    }

    #[test]
    fn finds_email_addresses() {
        assert_eq!(emails("contact someone@example.org now"), vec![8]);
        assert_eq!(
            emails("CRYPTOGAMS by <appro@openssl.org>"),
            Vec::<usize>::new()
        );
        assert_eq!(emails("git@github.com:T-G0W/x"), vec![0]);
        for text in ["@media screen", "a@b", "value@0x1f.20", "$%@.IQ", "x@a..de"] {
            assert_eq!(emails(text), Vec::<usize>::new(), "{text}");
        }
    }

    #[test]
    fn extracts_ascii_and_utf16_runs() {
        let bytes = b"\x00\x01hello world\x02ab\x00";
        let runs = ascii_runs(bytes);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].offset, 2);
        assert_eq!(runs[0].text, "hello world");
        let wide: Vec<u8> = "\u{1}T-GOW Launcher"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let runs = utf16_runs(&wide);
        assert!(runs.iter().any(|run| run.text == "T-GOW Launcher"));
    }
}
