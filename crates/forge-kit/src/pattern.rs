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

#[cfg(any(test, feature = "testing"))]
pub fn statics_declared_under(dir: &std::path::Path) -> std::io::Result<usize> {
    let mut count = 0;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            count += statics_declared_under(&path)?;
            continue;
        }
        count += std::fs::read_to_string(&path)?
            .lines()
            .map(|line| line.trim_start().trim_start_matches("pub(crate) "))
            .filter(|line| line.starts_with("static ") && line.contains(": Pattern ="))
            .count();
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::git::remote::{GITHUB_URL, PLAIN_HTTPS};

    const ALL: &[&Pattern] = &[&GITHUB_URL, &PLAIN_HTTPS];

    #[test]
    fn every_constant_pattern_compiles() {
        for pattern in ALL {
            assert!(pattern.is_match("").is_ok(), "{}", pattern.source);
        }
    }

    #[test]
    fn every_static_pattern_of_the_crate_is_listed() {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

        assert_eq!(statics_declared_under(&source).unwrap(), ALL.len());
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
