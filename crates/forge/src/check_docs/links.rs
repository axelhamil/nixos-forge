use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use super::markdown::{Link, Markdown};
use super::paths::{join, normalize, parent, percent_decode, percent_encode};
use super::{CheckDocsError, Findings, Page, Tree, read};

const MISSING_FILE: &str = "File not found. Check if file exists and path is correct";
const MISSING_FRAGMENT: &str = "Cannot find fragment";

struct Target {
    path: String,
    fragment: Option<String>,
}

impl Target {
    fn parse(page: &Page, link: &str) -> Option<Self> {
        let link = link.trim();
        if link.is_empty() || has_scheme(link) {
            return None;
        }

        let (location, fragment) = match link.split_once('#') {
            Some((location, fragment)) => (location, Some(fragment.to_owned())),
            None => (link, None),
        };
        let location = location.split_once('?').map_or(location, |(path, _)| path);

        let path = if location.is_empty() {
            page.path.clone()
        } else if let Some(absolute) = location.strip_prefix('/') {
            normalize(&percent_decode(absolute))
        } else {
            normalize(&join(parent(&page.path), &percent_decode(location)))
        };

        Some(Self { path, fragment })
    }

    fn display(&self) -> String {
        match &self.fragment {
            Some(fragment) => format!("{}#{fragment}", percent_encode(&self.path)),
            None => percent_encode(&self.path),
        }
    }
}

struct Resolver<'a> {
    root: &'a Path,
    markdown: Markdown,
    anchors: HashMap<String, BTreeSet<String>>,
}

impl Resolver<'_> {
    fn problem(&mut self, target: &Target) -> Result<Option<&'static str>, CheckDocsError> {
        let file = self.root.join(&target.path);
        if !file.exists() {
            return Ok(Some(MISSING_FILE));
        }

        let Some(fragment) = target
            .fragment
            .as_deref()
            .filter(|fragment| !fragment.is_empty())
        else {
            return Ok(None);
        };
        let page = if file.is_dir() {
            let readme = join(&target.path, "README.md");
            if !self.root.join(&readme).is_file() {
                return Ok(Some(MISSING_FRAGMENT));
            }
            readme
        } else if file.extension().is_some_and(|ext| ext == "md") {
            target.path.clone()
        } else {
            return Ok(None);
        };

        let anchors = self.anchors_of(&page)?;
        let decoded = percent_decode(fragment);
        let found = [fragment.to_owned(), decoded.to_lowercase(), decoded]
            .iter()
            .any(|candidate| anchors.contains(candidate));
        Ok((!found).then_some(MISSING_FRAGMENT))
    }

    fn anchors_of(&mut self, path: &str) -> Result<&BTreeSet<String>, CheckDocsError> {
        if !self.anchors.contains_key(path) {
            let text = read(&self.root.join(path))?;
            let anchors = self.markdown.anchors(&text);
            self.anchors.insert(path.to_owned(), anchors);
        }
        Ok(&self.anchors[path])
    }
}

pub fn check(tree: &Tree, findings: &mut Findings) -> Result<(), CheckDocsError> {
    let mut resolver = Resolver {
        root: &tree.root,
        markdown: Markdown::new()?,
        anchors: HashMap::new(),
    };

    for page in tree.pages() {
        for Link { target, line } in resolver.markdown.links(&page.text) {
            let Some(target) = Target::parse(page, &target) else {
                continue;
            };
            let Some(reason) = resolver.problem(&target)? else {
                continue;
            };
            let shown = target.display();
            findings.record(
                format!("link {} {shown}", page.path),
                format!("{}:{line}: broken link to {shown} ({reason})", page.path),
            );
        }
    }
    Ok(())
}

fn has_scheme(link: &str) -> bool {
    let Some((scheme, _)) = link.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}
