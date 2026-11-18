use std::io;

use zeroize::Zeroizing;

pub trait Terminal {
    fn line(&mut self, prompt: &str) -> io::Result<String>;
    fn secret(&mut self, prompt: &str) -> io::Result<Zeroizing<String>>;
    fn is_interactive(&self) -> bool;
}

#[cfg(any(test, feature = "testing"))]
pub struct Typed {
    pub answers: std::collections::VecDeque<io::Result<String>>,
    pub prompts: Vec<String>,
}

#[cfg(any(test, feature = "testing"))]
impl Typed {
    #[must_use]
    pub fn new(answers: &[&str]) -> Self {
        Self {
            answers: answers
                .iter()
                .map(|&answer| Ok(answer.to_owned()))
                .collect(),
            prompts: Vec::new(),
        }
    }

    #[must_use]
    pub fn then_fail(mut self, kind: io::ErrorKind) -> Self {
        self.answers.push_back(Err(kind.into()));
        self
    }
}

#[cfg(any(test, feature = "testing"))]
impl Terminal for Typed {
    fn line(&mut self, prompt: &str) -> io::Result<String> {
        self.prompts.push(prompt.to_owned());
        self.answers
            .pop_front()
            .unwrap_or_else(|| Err(io::ErrorKind::UnexpectedEof.into()))
    }

    fn secret(&mut self, prompt: &str) -> io::Result<Zeroizing<String>> {
        self.line(prompt).map(Zeroizing::new)
    }

    fn is_interactive(&self) -> bool {
        true
    }
}
