use std::fs;

use super::{Findings, Tree};

const KINDS: [(&str, &str); 4] = [
    ("app", "modules/apps"),
    ("feature", "modules/features"),
    ("hardware", "modules/features/hardware"),
    ("profile", "modules/features/profiles"),
];

const NOT_FEATURES: [&str; 3] = ["base", "profiles", "hardware"];

struct Coverage<'a> {
    headings: Vec<&'a str>,
    rows: Vec<&'a str>,
}

impl Coverage<'_> {
    fn names(&self, name: &str) -> bool {
        let quoted = format!("`{name}`");
        self.headings
            .iter()
            .any(|heading| heading.contains(&quoted))
            || self
                .rows
                .iter()
                .any(|row| first_cell(row).is_some_and(|cell| cell.contains(&quoted)))
    }
}

pub fn check(tree: &Tree, findings: &mut Findings) {
    let lines: Vec<&str> = tree
        .docs
        .iter()
        .flat_map(|page| page.text.lines())
        .collect();
    let coverage = Coverage {
        headings: lines
            .iter()
            .copied()
            .filter(|line| is_heading(line))
            .collect(),
        rows: lines
            .iter()
            .copied()
            .filter(|line| line.starts_with('|'))
            .collect(),
    };

    for (kind, dir) in KINDS {
        for name in modules_of(tree, dir) {
            if kind == "feature" && NOT_FEATURES.contains(&name.as_str()) {
                continue;
            }
            if coverage.names(&name) {
                continue;
            }
            findings.record(
                format!("section {kind} {name}"),
                format!(
                    "docs/: no heading or first table cell names the {kind} `{name}` ({dir}/{name})"
                ),
            );
        }
    }
}

fn is_heading(line: &str) -> bool {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    (1..=6).contains(&hashes) && line[hashes..].starts_with(' ')
}

fn first_cell(row: &str) -> Option<&str> {
    let rest = row.strip_prefix('|')?;
    rest.find('|').map(|end| &rest[..end])
}

fn modules_of(tree: &Tree, dir: &str) -> Vec<String> {
    let Ok(entries) = fs::read_dir(tree.root.join(dir)) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}
