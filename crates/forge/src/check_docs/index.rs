use std::collections::BTreeSet;

use super::paths::{join, normalize};
use super::{CheckDocsError, Findings, Tree, compile};

const INDEX: &str = "docs/README.md";

pub fn check(tree: &Tree, findings: &mut Findings) -> Result<(), CheckDocsError> {
    let Some(index) = tree.docs.iter().find(|page| page.path == INDEX) else {
        return Ok(());
    };

    let link = compile(r"\]\(([^)#\s]+)")?;
    let indexed: BTreeSet<String> = link
        .captures_iter(&index.text)
        .flatten()
        .filter_map(|captures| captures.get(1))
        .map(|target| normalize(&join("docs", target.as_str())))
        .collect();

    for page in tree.docs.iter().filter(|page| page.path != INDEX) {
        if indexed.contains(&page.path) {
            continue;
        }
        findings.record(
            format!("orphan {}", page.path),
            format!("{}: no link to it in {INDEX}, the docs index", page.path),
        );
    }
    Ok(())
}
