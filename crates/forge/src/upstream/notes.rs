use std::cmp::Ordering;

use fancy_regex::Regex;

use crate::system::remote::{GITHUB, UPSTREAM_REPOSITORY, github_repository};

const GITHUB_SHORTHAND: &str = r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$";
const BREAKING_LINE: &str = r"^(BREAKING[ -]CHANGE:|[a-z]+(\([^)]*\))?!:)";
const TAGS_PREFIX: &str = "refs/tags/";
const PEELED_SUFFIX: &str = "^{}";

#[must_use]
pub fn starter() -> String {
    format!("{GITHUB}{UPSTREAM_REPOSITORY}.git")
}

pub(super) fn matches(pattern: &str, text: &str) -> bool {
    Regex::new(pattern).is_ok_and(|regex| regex.is_match(text).unwrap_or(false))
}

#[must_use]
pub fn source_url(source: &str) -> String {
    if matches(GITHUB_SHORTHAND, source) {
        return format!("{GITHUB}{source}.git");
    }
    source.to_owned()
}

#[must_use]
pub fn releases_page(url: &str) -> Option<String> {
    github_repository(url).map(|repository| format!("{GITHUB}{repository}/releases/tag"))
}

#[must_use]
pub fn latest_tag(ls_remote: &str) -> Option<String> {
    ls_remote
        .lines()
        .filter(|line| !line.ends_with(PEELED_SUFFIX))
        .filter_map(|line| line.rsplit_once(TAGS_PREFIX).map(|(_, tag)| tag))
        .filter(|tag| is_release(tag))
        .max_by(|left, right| version_order(left, right))
        .map(str::to_owned)
}

fn is_release(tag: &str) -> bool {
    tag.strip_prefix('v')
        .and_then(|rest| rest.chars().next())
        .is_some_and(|first| first.is_ascii_digit())
}

fn version_order(left: &str, right: &str) -> Ordering {
    let (left_chunks, right_chunks) = (chunks(left), chunks(right));
    left_chunks
        .iter()
        .zip(&right_chunks)
        .map(|(left, right)| chunk_order(left, right))
        .find(|order| order.is_ne())
        .unwrap_or_else(|| left_chunks.len().cmp(&right_chunks.len()))
        .then_with(|| left.cmp(right))
}

fn chunks(text: &str) -> Vec<&str> {
    let mut chunks = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for index in 1..bytes.len() {
        if bytes[index].is_ascii_digit() != bytes[index - 1].is_ascii_digit() {
            chunks.push(&text[start..index]);
            start = index;
        }
    }
    if start < text.len() {
        chunks.push(&text[start..]);
    }
    chunks
}

fn chunk_order(left: &str, right: &str) -> Ordering {
    let numeric = |chunk: &str| chunk.bytes().all(|byte| byte.is_ascii_digit());
    if !(numeric(left) && numeric(right)) {
        return left.cmp(right);
    }
    let (left, right) = (left.trim_start_matches('0'), right.trim_start_matches('0'));
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

#[must_use]
pub fn breaking_lines(log: &str) -> String {
    let Ok(breaking) = Regex::new(BREAKING_LINE) else {
        return String::new();
    };
    log.lines()
        .filter(|line| breaking.is_match(line).unwrap_or(false))
        .collect::<Vec<_>>()
        .join("\n")
}

#[must_use]
pub fn changelog_lines(diff: &str) -> String {
    let added: Vec<&str> = diff
        .lines()
        .filter(|line| !line.starts_with("+++"))
        .filter_map(|line| line.strip_prefix('+'))
        .skip_while(|line| line.is_empty())
        .collect();
    added.join("\n").trim_end_matches('\n').to_owned()
}

#[must_use]
pub fn notes(breaking: &str, changelog: &str) -> String {
    let changelog = if changelog.is_empty() {
        "No CHANGELOG.md entry."
    } else {
        changelog
    };
    let changes = format!("Changelog:\n{changelog}");
    if breaking.is_empty() {
        return changes;
    }
    format!("Breaking changes:\n{breaking}\n\n{changes}")
}

#[must_use]
pub fn pull_request_body(merged: &str, commits: &str, notes: &str) -> String {
    format!(
        "Merges {merged} into this fork. Review the diff before merging it with a merge commit, never a squash.

Upstream commits merged:

```
{commits}```

```
{notes}
```
"
    )
}

#[must_use]
pub fn merged_release(url: &str, tag: &str) -> String {
    match releases_page(url) {
        Some(releases) => format!("[{tag}]({releases}/{tag})"),
        None => url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_owner_and_repo_means_a_github_repository_and_anything_else_is_kept_as_written() {
        assert_eq!(
            source_url("bob/my-nixos"),
            "https://github.com/bob/my-nixos.git"
        );
        assert_eq!(source_url("file:///srv/starter"), "file:///srv/starter");
        assert_eq!(source_url("/srv/git/starter"), "/srv/git/starter");
        assert_eq!(starter(), "https://github.com/axelhamil/nixos-forge.git");
    }

    #[test]
    fn only_a_github_starter_has_a_releases_page() {
        assert_eq!(
            releases_page("https://github.com/bob/forge.git").as_deref(),
            Some("https://github.com/bob/forge/releases/tag")
        );
        assert_eq!(releases_page("file:///srv/starter"), None);
    }

    #[test]
    fn the_latest_release_tag_sorts_by_version_and_skips_peeled_and_other_tags() {
        let ls_remote = "\
a\trefs/tags/v0.9.0
b\trefs/tags/v0.10.0
c\trefs/tags/v0.10.0^{}
d\trefs/tags/zzz
e\trefs/tags/nightly
f\trefs/tags/nightly^{}
g\trefs/tags/v0.2.0
";

        assert_eq!(latest_tag(ls_remote).as_deref(), Some("v0.10.0"));
        assert_eq!(latest_tag("d\trefs/tags/zzz\n"), None);
        assert_eq!(latest_tag(""), None);
    }

    #[test]
    fn breaking_lines_are_the_bang_subjects_and_breaking_change_footers() {
        let log = "\
feat!: rename x

fix(disk)!: drop the old label
feat: harmless
  feat!: indented
BREAKING CHANGE: x is y now
BREAKING-CHANGE: also
Feat!: capital
";

        assert_eq!(
            breaking_lines(log),
            "feat!: rename x\nfix(disk)!: drop the old label\nBREAKING CHANGE: x is y now\nBREAKING-CHANGE: also"
        );
    }

    #[test]
    fn the_changelog_is_the_added_lines_of_its_diff_without_leading_or_trailing_blank_lines() {
        let diff = "\
diff --git a/CHANGELOG.md b/CHANGELOG.md
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -1 +1,5 @@
 # Changelog
+
+## 0.2.0
+
+* rename x to y
+
-old
";

        assert_eq!(changelog_lines(diff), "## 0.2.0\n\n* rename x to y");
        assert_eq!(changelog_lines(""), "");
    }

    #[test]
    fn the_notes_list_the_breaking_changes_first_and_say_when_the_changelog_is_empty() {
        assert_eq!(
            notes("feat!: rename x", "* rename x to y"),
            "Breaking changes:\nfeat!: rename x\n\nChangelog:\n* rename x to y"
        );
        assert_eq!(notes("", ""), "Changelog:\nNo CHANGELOG.md entry.");
    }

    #[test]
    fn the_merged_release_links_its_github_page_or_names_the_starter() {
        assert_eq!(
            merged_release("https://github.com/bob/forge.git", "v0.2.0"),
            "[v0.2.0](https://github.com/bob/forge/releases/tag/v0.2.0)"
        );
        assert_eq!(
            merged_release("file:///srv/starter", "v0.2.0"),
            "file:///srv/starter"
        );
    }

    #[test]
    fn the_pull_request_body_says_what_it_merges_then_lists_the_commits_and_the_notes() {
        let body = pull_request_body(
            "[v0.2.0](https://github.com/bob/forge/releases/tag/v0.2.0)",
            "1111111 chore: release 0.2.0\n2222222 feat!: rename x\n",
            "Changelog:\nNo CHANGELOG.md entry.",
        );

        assert_eq!(
            body,
            "Merges [v0.2.0](https://github.com/bob/forge/releases/tag/v0.2.0) into this fork. Review the diff before merging it with a merge commit, never a squash.

Upstream commits merged:

```
1111111 chore: release 0.2.0
2222222 feat!: rename x
```

```
Changelog:
No CHANGELOG.md entry.
```
"
        );
    }
}
