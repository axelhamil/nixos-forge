use crate::invocation::Invocation;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answer {
    pub success: bool,
    pub code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl Answer {
    #[must_use]
    pub fn ok(stdout: &str) -> Self {
        Self {
            success: true,
            code: Some(0),
            stdout: stdout.to_owned(),
            stderr: String::new(),
        }
    }

    #[must_use]
    pub fn failed(stderr: &str) -> Self {
        Self {
            success: false,
            code: None,
            stdout: String::new(),
            stderr: stderr.to_owned(),
        }
    }

    #[must_use]
    pub fn exited(code: i32) -> Self {
        Self {
            success: code == 0,
            code: Some(code),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn exit_code(&self) -> u8 {
        let code = self
            .code
            .and_then(|code| u8::try_from(code).ok())
            .filter(|&code| code != 0);
        match code {
            Some(code) => code,
            None if self.success => 0,
            None => 1,
        }
    }

    #[must_use]
    pub fn line(&self) -> &str {
        self.stdout.trim_end_matches('\n')
    }
}

pub trait Commands {
    fn run(&self, invocation: &Invocation) -> Answer;
}
