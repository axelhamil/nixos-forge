use std::sync::OnceLock;

use regex::{Captures, Regex};

pub struct Pattern {
    source: &'static str,
    compiled: OnceLock<Result<Regex, regex::Error>>,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("invalid pattern {source_text}: {error}")]
pub struct PatternInvalid {
    source_text: &'static str,
    error: regex::Error,
}

impl Pattern {
    #[must_use]
    pub const fn new(source: &'static str) -> Self {
        Self {
            source,
            compiled: OnceLock::new(),
        }
    }

    pub fn is_match(&self, text: &str) -> Result<bool, PatternInvalid> {
        Ok(self.compiled()?.is_match(text))
    }

    pub fn captures<'t>(&self, text: &'t str) -> Result<Option<Captures<'t>>, PatternInvalid> {
        Ok(self.compiled()?.captures(text))
    }

    fn compiled(&self) -> Result<&Regex, PatternInvalid> {
        self.compiled
            .get_or_init(|| Regex::new(self.source))
            .as_ref()
            .map_err(|error| PatternInvalid {
                source_text: self.source,
                error: error.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::auto_upgrade::sources::{SCP_LIKE, SSH_URL};
    use crate::init::answers::{EMAIL, HOST_NAME, LIST_ITEM, LOCALE, LOGIN, WORD};
    use crate::init::render::{BARE_ATTRIBUTE, BLOCK_ITEM, BLOCK_START, FLAKE, SIGNERS};
    use crate::system::remote::{GITHUB_URL, PLAIN_HTTPS};
    use crate::upstream::notes::{BREAKING_LINE, GITHUB_SHORTHAND};

    const ALL: &[&Pattern] = &[
        &SSH_URL,
        &SCP_LIKE,
        &EMAIL,
        &HOST_NAME,
        &LIST_ITEM,
        &LOCALE,
        &LOGIN,
        &WORD,
        &BARE_ATTRIBUTE,
        &BLOCK_ITEM,
        &BLOCK_START,
        &FLAKE,
        &SIGNERS,
        &GITHUB_URL,
        &PLAIN_HTTPS,
        &BREAKING_LINE,
        &GITHUB_SHORTHAND,
    ];

    #[test]
    fn every_constant_pattern_compiles() {
        for pattern in ALL {
            assert!(pattern.is_match("").is_ok(), "{}", pattern.source);
        }
    }

    fn statics_declared_under(dir: &Path) -> usize {
        fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .map(|path| {
                if path.is_dir() {
                    return statics_declared_under(&path);
                }
                fs::read_to_string(&path)
                    .unwrap()
                    .lines()
                    .map(|line| line.trim_start().trim_start_matches("pub(crate) "))
                    .filter(|line| line.starts_with("static ") && line.contains(": Pattern ="))
                    .count()
            })
            .sum()
    }

    #[test]
    fn every_static_pattern_of_the_crate_is_listed() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

        assert_eq!(statics_declared_under(&source), ALL.len());
    }

    #[test]
    fn an_invalid_pattern_is_an_error_not_a_miss() {
        assert!(Pattern::new("(").is_match("x").is_err());
    }

    #[test]
    fn a_valid_pattern_matches_and_captures() {
        let pattern = Pattern::new(r"^a(b+)$");

        assert_eq!(pattern.is_match("abb").ok(), Some(true));
        assert_eq!(pattern.is_match("ac").ok(), Some(false));
        let captured = pattern
            .captures("abbb")
            .unwrap()
            .and_then(|captures| captures.get(1).map(|group| group.as_str()));
        assert_eq!(captured, Some("bbb"));
    }

    #[test]
    fn an_invalid_pattern_names_itself() {
        let error = Pattern::new("(").is_match("x").unwrap_err();

        assert!(error.to_string().starts_with("invalid pattern ("));
    }
}
