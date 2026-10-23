mod index;
mod links;
mod markdown;
mod names;
mod paths;
mod sections;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

pub const PENDING_FILE: &str = "modules/flake/docs/pending.txt";

#[derive(Debug, thiserror::Error)]
pub enum CheckDocsError {
    #[error("cannot open the root {path}: {source}")]
    Root {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot list {path}: {source}")]
    Walk {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },
    #[error("invalid pattern {pattern}: {source}")]
    Pattern {
        pattern: String,
        #[source]
        source: Box<fancy_regex::Error>,
    },
}

#[derive(Debug, Default)]
struct Findings(BTreeMap<String, Vec<String>>);

impl Findings {
    fn record(&mut self, key: String, message: String) {
        self.0.entry(key).or_default().push(message);
    }
}

struct Page {
    path: String,
    text: String,
}

struct Tree {
    root: PathBuf,
    readme: Option<Page>,
    docs: Vec<Page>,
}

#[derive(Debug)]
pub struct Outcome {
    pub problems: Vec<String>,
    pub pending: usize,
}

impl Outcome {
    #[must_use]
    pub fn passed(&self) -> bool {
        self.problems.is_empty()
    }

    #[must_use]
    pub fn summary(&self) -> String {
        if self.passed() {
            return format!(
                "check-docs: docs pass, {} pending entries in {PENDING_FILE}",
                self.pending
            );
        }
        format!(
            "check-docs: {} problem(s); a known violation waiting for its fix goes in {PENDING_FILE} as \"<rule> <what>\"",
            self.problems.len()
        )
    }

    #[must_use]
    pub fn report(&self) -> String {
        let mut lines = self.problems.clone();
        lines.push(self.summary());
        lines.join("\n") + "\n"
    }
}

pub fn run(root: &Path) -> Result<Outcome, CheckDocsError> {
    let tree = Tree::load(root)?;
    let mut findings = Findings::default();

    links::check(&tree, &mut findings)?;
    index::check(&tree, &mut findings)?;
    names::check(&tree, &mut findings)?;
    sections::check(&tree, &mut findings);

    let pending = read_pending(&tree.root)?;
    Ok(judge(&findings, &pending))
}

fn judge(findings: &Findings, pending: &BTreeSet<String>) -> Outcome {
    let unexpected = findings
        .0
        .iter()
        .filter(|(key, _)| !pending.contains(*key))
        .map(|(key, messages)| format!("{} [{key}]", messages.join("\n")));

    let stale = pending
        .iter()
        .filter(|entry| !findings.0.contains_key(*entry))
        .map(|entry| format!("{PENDING_FILE}: \"{entry}\" no longer fails: remove its line"));

    Outcome {
        problems: unexpected.chain(stale).collect(),
        pending: pending.len(),
    }
}

fn read_pending(root: &Path) -> Result<BTreeSet<String>, CheckDocsError> {
    let path = root.join(PENDING_FILE);
    if !path.is_file() {
        return Ok(BTreeSet::new());
    }

    let text = read(&path)?;
    Ok(text
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect())
}

impl Tree {
    fn load(root: &Path) -> Result<Self, CheckDocsError> {
        let root = fs::canonicalize(root).map_err(|source| CheckDocsError::Root {
            path: root.to_path_buf(),
            source,
        })?;

        let readme = root
            .join("README.md")
            .is_file()
            .then(|| Page::load(&root, "README.md"))
            .transpose()?;

        let docs = markdown_files(&root, "docs")?
            .into_iter()
            .map(|path| Page::load(&root, &path))
            .collect::<Result<_, _>>()?;

        Ok(Self { root, readme, docs })
    }

    fn pages(&self) -> impl Iterator<Item = &Page> {
        self.readme.iter().chain(&self.docs)
    }
}

impl Page {
    fn load(root: &Path, path: &str) -> Result<Self, CheckDocsError> {
        Ok(Self {
            path: path.to_owned(),
            text: read(&root.join(path))?,
        })
    }
}

fn read(path: &Path) -> Result<String, CheckDocsError> {
    fs::read_to_string(path).map_err(|source| CheckDocsError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn markdown_files(root: &Path, dir: &str) -> Result<Vec<String>, CheckDocsError> {
    let base = root.join(dir);
    if !base.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(&base) {
        let entry = entry.map_err(|source| CheckDocsError::Walk {
            path: base.clone(),
            source,
        })?;
        if !entry.file_type().is_file() || entry.path().extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        if let Ok(relative) = entry.path().strip_prefix(root) {
            files.push(paths::to_slash(relative));
        }
    }
    files.sort();
    Ok(files)
}

fn compile(pattern: &str) -> Result<fancy_regex::Regex, CheckDocsError> {
    fancy_regex::Regex::new(pattern).map_err(|source| CheckDocsError::Pattern {
        pattern: pattern.to_owned(),
        source: Box::new(source),
    })
}
