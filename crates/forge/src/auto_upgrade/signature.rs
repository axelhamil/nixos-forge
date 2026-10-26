use std::path::Path;

use crate::system::{Answer, Git};

pub struct Signers<'a> {
    git: &'a Git<'a>,
    verifying: Git<'a>,
    host: &'a str,
    allowed: String,
}

impl<'a> Signers<'a> {
    #[must_use]
    pub fn new(git: &'a Git<'a>, host: &'a str, allowed: &Path) -> Self {
        Self {
            git,
            verifying: git
                .with_env("GIT_CONFIG_NOSYSTEM", "1")
                .with_env("GIT_CONFIG_GLOBAL", "/dev/null"),
            host,
            allowed: format!("gpg.ssh.allowedSignersFile={}", allowed.display()),
        }
    }

    #[must_use]
    pub fn refusal(&self, commit: &str) -> Option<&'static str> {
        if self.signers_git(&["verify-commit", commit], false).success {
            let subject = self.git.loud(&["log", "-1", "--format=%s", commit]);
            if subject
                .line()
                .starts_with(&format!("deploy {}: ", self.host))
            {
                return None;
            }
            return Some("signed for another host");
        }

        let status = self.signers_git(&["log", "-1", "--format=%G?", commit], true);
        Some(match status.line() {
            "N" => {
                let raw = self.git.loud(&["cat-file", "commit", commit]);
                if raw.line().contains("\ngpgsig ") {
                    "not signed with an SSH key"
                } else {
                    "unsigned"
                }
            }
            "B" => "signature invalid",
            _ => "signed by a key not in update.signers",
        })
    }

    fn signers_git(&self, args: &[&str], shown: bool) -> Answer {
        let args: Vec<&str> = [
            "-c",
            "gpg.format=ssh",
            "-c",
            &self.allowed,
            "-c",
            "gpg.openpgp.program=false",
            "-c",
            "gpg.x509.program=false",
        ]
        .into_iter()
        .chain(args.iter().copied())
        .collect();
        if shown {
            return self.verifying.loud(&args);
        }
        self.verifying.quiet(&args)
    }
}
