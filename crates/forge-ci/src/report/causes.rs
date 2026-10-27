use fancy_regex::Regex;

const TABLE: &str = include_str!("causes.tsv");

#[derive(Debug, thiserror::Error)]
pub enum CausesError {
    #[error("line {line} of causes.tsv has no pattern, cause and todo")]
    Fields { line: usize },
    #[error("line {line} of causes.tsv has an invalid pattern: {source}")]
    Pattern {
        line: usize,
        source: Box<fancy_regex::Error>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Diagnosis<'a> {
    pub cause: &'a str,
    pub todo: &'a str,
}

#[derive(Debug)]
struct Cause {
    pattern: Regex,
    diagnosis: Diagnosis<'static>,
}

#[derive(Debug)]
pub struct Causes(Vec<Cause>);

impl Causes {
    pub fn embedded() -> Result<Self, CausesError> {
        TABLE
            .split('\n')
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
            .map(|(index, line)| parse(index + 1, line))
            .collect::<Result<_, _>>()
            .map(Self)
    }

    #[must_use]
    pub fn diagnose(&self, excerpt: &[String]) -> Option<Diagnosis<'static>> {
        self.0
            .iter()
            .find(|cause| {
                excerpt
                    .iter()
                    .any(|line| cause.pattern.is_match(line).unwrap_or(false))
            })
            .map(|cause| cause.diagnosis)
    }
}

fn parse(line: usize, text: &'static str) -> Result<Cause, CausesError> {
    let mut fields = text.split('\t');
    let (Some(pattern), Some(cause), Some(todo)) = (fields.next(), fields.next(), fields.next())
    else {
        return Err(CausesError::Fields { line });
    };

    let pattern = Regex::new(pattern).map_err(|source| CausesError::Pattern {
        line,
        source: Box::new(source),
    })?;
    Ok(Cause {
        pattern,
        diagnosis: Diagnosis { cause, todo },
    })
}
