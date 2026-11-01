use ra_ap_rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};

use crate::repo::{SourceFile, extension};
use crate::report::Finding;

const SUFFIXES: [&str; 14] = [
    "Manager",
    "Helper",
    "Helpers",
    "Util",
    "Utils",
    "Handler",
    "Processor",
    "Service",
    "Factory",
    "Wrapper",
    "Impl",
    "Base",
    "Data",
    "Info",
];

const MODULES: [&str; 7] = [
    "utils", "helpers", "common", "misc", "shared", "stuff", "manager",
];

const ALLOWED: [&str; 0] = [];

const DECLARATIONS: [&str; 5] = ["struct", "enum", "trait", "type", "union"];

pub(crate) fn check(files: &[SourceFile]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for file in files.iter().filter(|file| extension(&file.path) == "rs") {
        findings.extend(check_path(&file.path));
        if let Some(text) = &file.text {
            findings.extend(check_source(&file.path, text));
        }
    }
    findings
}

fn check_path(path: &str) -> Option<Finding> {
    path.split('/')
        .map(|component| component.strip_suffix(".rs").unwrap_or(component))
        .find(|stem| MODULES.contains(stem))
        .map(|stem| Finding::whole(path, format!("module name `{stem}` says nothing")))
}

fn check_source(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut previous: Option<&str> = None;
    let mut offset = 0;
    for token in tokenize(text, FrontmatterAllowed::No) {
        let end = offset + token.len as usize;
        let word = text.get(offset..end).unwrap_or_default();
        match token.kind {
            TokenKind::Whitespace
            | TokenKind::LineComment { .. }
            | TokenKind::BlockComment { .. } => {}
            TokenKind::Ident | TokenKind::RawIdent => {
                let name = word.trim_start_matches("r#");
                match previous {
                    Some(keyword) if DECLARATIONS.contains(&keyword) => {
                        if let Some(message) = type_name(name) {
                            findings.push(Finding::at(path, text, offset, message));
                        }
                    }
                    Some("mod") if MODULES.contains(&name) => {
                        findings.push(Finding::at(
                            path,
                            text,
                            offset,
                            format!("module name `{name}` says nothing"),
                        ));
                    }
                    _ => {}
                }
                previous = Some(word);
            }
            _ => previous = None,
        }
        offset = end;
    }
    findings
}

fn type_name(name: &str) -> Option<String> {
    if ALLOWED.contains(&name) {
        return None;
    }
    SUFFIXES
        .iter()
        .find(|suffix| name.ends_with(*suffix))
        .map(|suffix| format!("type `{name}` ends with `{suffix}`, name the domain concept"))
}

#[cfg(test)]
mod tests {
    use super::{check_path, check_source};

    #[test]
    fn rejects_suffix_soup_in_declarations() {
        let cases = [
            ("struct PlaybackManager;", true),
            ("pub(crate) enum FrameHelper { A }", true),
            ("trait AudioUtils {}", true),
            ("type EventHandler = u8;", true),
            ("struct PacketData;", true),
            ("struct Info;", true),
            ("union RawImpl { a: u8 }", true),
            ("struct Demuxer;", false),
            ("struct Metadata;", false),
            ("struct Database;", false),
            ("let manager_data = 1;", false),
            ("fn handler() {}", false),
            ("impl Iterator for A { type Item = u8; }", false),
        ];
        for (text, rejected) in cases {
            assert_eq!(
                !check_source("src/a.rs", text).is_empty(),
                rejected,
                "{text}"
            );
        }
    }

    #[test]
    fn rejects_catch_all_modules() {
        assert_eq!(
            check_source("src/lib.rs", "mod utils;\nmod demux;\n").len(),
            1
        );
        assert!(check_path("crates/engine/src/common.rs").is_some());
        assert!(check_path("crates/engine/src/helpers/clock.rs").is_some());
        assert!(check_path("crates/engine/src/clock.rs").is_none());
        assert!(check_path("crates/engine/src/commonality.rs").is_none());
    }

    #[test]
    fn reports_position_of_the_name() {
        let findings = check_source("src/a.rs", "\nstruct VideoInfo;\n");
        assert_eq!((findings[0].line, findings[0].column), (2, 8));
    }
}
