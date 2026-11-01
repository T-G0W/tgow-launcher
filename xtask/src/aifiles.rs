use crate::report::Finding;

const DIRECTORIES: [&str; 4] = [".claude", ".cursor", ".windsurf", "prompts"];

const FILES: [&str; 10] = [
    "claude.md",
    "claude.local.md",
    "agents.md",
    "gemini.md",
    ".cursorrules",
    ".windsurfrules",
    ".mcp.json",
    "copilot-instructions.md",
    "prompt.txt",
    ".aiderignore",
];

pub(crate) fn check(paths: &[String]) -> Vec<Finding> {
    paths
        .iter()
        .filter(|path| is_ai_file(path))
        .map(|path| Finding::whole(path, "ai tool file must stay outside the repository"))
        .collect()
}

fn is_ai_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let mut components = lower.split('/').peekable();
    while let Some(component) = components.next() {
        let is_file = components.peek().is_none();
        if is_file {
            return FILES.contains(&component)
                || component.starts_with(".aider")
                || component.ends_with(".prompt")
                || component.ends_with(".prompt.md");
        }
        if DIRECTORIES.contains(&component) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_ai_file;

    #[test]
    fn recognizes_ai_tool_files() {
        let cases = [
            ("CLAUDE.md", true),
            ("docs/CLAUDE.local.md", true),
            (".claude/settings.json", true),
            ("AGENTS.md", true),
            (".github/copilot-instructions.md", true),
            (".cursor/rules/a.mdc", true),
            (".mcp.json", true),
            (".aider.conf.yml", true),
            ("review.prompt.md", true),
            ("Prompt.txt", true),
            ("README.md", false),
            ("crates/engine/src/claude.rs", false),
            ("TRADEMARKS.md", false),
        ];
        for (path, expected) in cases {
            assert_eq!(is_ai_file(path), expected, "{path}");
        }
    }
}
