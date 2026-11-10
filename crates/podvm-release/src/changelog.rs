//! The section of `CHANGELOG.md` that a release publishes (T-1671).
//!
//! A section starts at a line whose first two words are `##` and the
//! version, and ends at the next `## ` line. `release.yml` finds the section
//! with the same rule before it creates the tag, so the check and the notes
//! cannot disagree about which section exists.
//!
//! The release notes lift each `### ` entry heading to `## `: on the release
//! page an entry is a top-level section, and `### Breaking` becomes
//! `## Breaking`.

/// True where `line` is the heading of the section of `version`.
pub fn is_heading(line: &str, version: &str) -> bool {
    let mut words = line.split_whitespace();
    words.next() == Some("##") && words.next() == Some(version)
}

/// The body of the section of `version`, with each `### ` heading lifted to
/// `## ` and the blank lines at each end removed. `None` where the changelog
/// has no section for `version`, or the section is empty.
pub fn release_section(changelog: &str, version: &str) -> Option<String> {
    let mut lines = changelog.lines();
    lines.by_ref().find(|line| is_heading(line, version))?;
    let mut body: Vec<String> = Vec::new();
    for line in lines {
        if line.starts_with("## ") {
            break;
        }
        match line.strip_prefix("### ") {
            Some(rest) => body.push(format!("## {rest}")),
            None => body.push(line.to_string()),
        }
    }
    while body.last().is_some_and(|line| line.trim().is_empty()) {
        body.pop();
    }
    let first = body.iter().position(|line| !line.trim().is_empty())?;
    Some(body[first..].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        "# Changelog\n\n## Unreleased\n\n### Later work, 2026-11-11\n\nRecord: a.\n\n\
        ## 0.2.0-beta.10 (2026-12-01)\n\n### Breaking: not this one, 2026-12-01\n\n\
        ## 0.2.0-beta.1 (2026-11-10)\n\n### Breaking: podbox is podvm, 2026-11-10\n\n\
        Record: the rename.\n\n- The binary is `podvm`.\n\n### Fixed, 2026-11-10\n\n- A fix.\n\n\
        ## 0.1.0-beta.13\n\n- Old text.\n";

    #[test]
    fn the_section_of_the_version_starts_with_breaking_as_a_top_heading() {
        let notes = release_section(FIXTURE, "0.2.0-beta.1").unwrap();
        assert!(notes.starts_with("## Breaking: podbox is podvm, 2026-11-10\n"));
        assert!(notes.lines().any(|l| l == "## Fixed, 2026-11-10"));
        assert!(notes.ends_with("- A fix."));
        assert!(!notes.contains("###"));
        assert!(!notes.contains("Old text"));
        assert!(!notes.contains("Later work"));
    }

    /// A version that is a prefix of another must not take that section.
    #[test]
    fn a_version_matches_its_own_heading_only() {
        assert!(is_heading("## 0.2.0-beta.1 (2026-11-10)", "0.2.0-beta.1"));
        assert!(is_heading("## 0.1.0-beta.13", "0.1.0-beta.13"));
        assert!(!is_heading("## 0.2.0-beta.10 (2026-12-01)", "0.2.0-beta.1"));
        assert!(!is_heading("### 0.2.0-beta.1", "0.2.0-beta.1"));
        let notes = release_section(FIXTURE, "0.2.0-beta.10").unwrap();
        assert_eq!(notes, "## Breaking: not this one, 2026-12-01");
    }

    #[test]
    fn no_section_is_none() {
        assert_eq!(release_section(FIXTURE, "0.3.0"), None);
        assert_eq!(release_section("## 0.3.0\n\n## 0.2.0\n", "0.3.0"), None);
    }

    /// The tree's changelog has a section for the workspace version: the
    /// release workflow refuses to tag without one, and so does this test.
    #[test]
    fn the_changelog_has_a_section_for_the_workspace_version() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let text = std::fs::read_to_string(root.join("CHANGELOG.md")).expect("CHANGELOG.md");
        let version = env!("CARGO_PKG_VERSION");
        assert!(
            release_section(&text, version).is_some(),
            "CHANGELOG.md has no `## {version}` section"
        );
    }
}
