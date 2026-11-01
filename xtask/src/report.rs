use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) message: String,
}

impl Finding {
    pub(crate) fn at(path: &str, text: &str, offset: usize, message: impl Into<String>) -> Self {
        let (line, column) = position(text, offset);
        Self {
            path: path.to_owned(),
            line,
            column,
            message: message.into(),
        }
    }

    pub(crate) fn whole(path: &str, message: impl Into<String>) -> Self {
        Self {
            path: path.to_owned(),
            line: 0,
            column: 0,
            message: message.into(),
        }
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}", self.path, self.message)
        } else {
            write!(
                f,
                "{}:{}:{}: {}",
                self.path, self.line, self.column, self.message
            )
        }
    }
}

pub(crate) fn position(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let column = before
        .get(line_start..)
        .map_or(0, |rest| rest.chars().count())
        + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::{Finding, position};

    #[test]
    fn counts_lines_and_columns_from_one() {
        let text = "ab\ncä d\n";
        let cases = [
            (0, (1, 1)),
            (1, (1, 2)),
            (3, (2, 1)),
            (6, (2, 3)),
            (7, (2, 4)),
        ];
        for (offset, expected) in cases {
            assert_eq!(position(text, offset), expected, "offset {offset}");
        }
    }

    #[test]
    fn formats_with_and_without_position() {
        let located = Finding::at("src/a.rs", "x\n// y", 2, "line comment");
        assert_eq!(located.to_string(), "src/a.rs:2:1: line comment");
        let whole = Finding::whole("CLAUDE.md", "ai file");
        assert_eq!(whole.to_string(), "CLAUDE.md: ai file");
    }
}
