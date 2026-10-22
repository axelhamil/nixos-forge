use fancy_regex::Regex;

use super::{CheckDocsError, Findings, Page, Tree, compile};

const FIDO2_PAM_ORIGIN: &str = "pam://nixos-forge";

const FORBIDDEN: [(&str, &str); 8] = [
    ("alice", r"\balice\b"),
    ("example-server", r"\bexample-server\b"),
    ("example-pi", r"\bexample-pi\b"),
    ("axelhamil", "axelhamil"),
    ("nixos-forge", "nixos-forge"),
    ("nixos-starter", "nixos-starter"),
    (
        "example",
        r"\x60example\x60|(?<![\w-])example(?= hosts?\b)|(?<![\w-])example-(?:disk|boots|toplevel|installer|wipe-home)(?![\w-])|deploy/example(?![\w-])",
    ),
    ("the starter", r"\bthe starter\b"),
];

struct Rule {
    name: &'static str,
    pattern: Regex,
    marker: String,
}

pub fn check(tree: &Tree, findings: &mut Findings) -> Result<(), CheckDocsError> {
    let rules = FORBIDDEN
        .iter()
        .map(|(name, pattern)| {
            Ok(Rule {
                name,
                pattern: compile(&format!("(?i){pattern}"))?,
                marker: format!("<!-- docs-check: allow {name} -->"),
            })
        })
        .collect::<Result<Vec<_>, CheckDocsError>>()?;

    let pages = tree
        .docs
        .iter()
        .filter(|page| !page.path.starts_with("docs/maintenance/"));
    for page in pages {
        check_page(page, &rules, findings);
    }
    Ok(())
}

fn check_page(page: &Page, rules: &[Rule], findings: &mut Findings) {
    let text = page.text.replace(FIDO2_PAM_ORIGIN, "");
    for rule in rules {
        let lines: Vec<String> = text
            .lines()
            .enumerate()
            .filter(|(_, line)| rule.pattern.is_match(line).unwrap_or(false))
            .filter(|(_, line)| !line.contains(&rule.marker))
            .map(|(index, _)| (index + 1).to_string())
            .collect();
        if lines.is_empty() {
            continue;
        }
        findings.record(
            format!("forbidden {} {}", page.path, rule.name),
            format!(
                "{}:{}: names \"{}\", which a fork does not have (or mark the line with {})",
                page.path,
                lines.join(","),
                rule.name,
                rule.marker
            ),
        );
    }
}
