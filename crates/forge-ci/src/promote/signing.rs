use std::ffi::OsStr;
use std::fs::{self, OpenOptions, Permissions};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use forge::system::{Commands, Git, Invocation};
use tempfile::TempDir;

use super::is_promotion;

const PRIVATE_DIR: u32 = 0o700;
const PRIVATE_FILE: u32 = 0o600;
const FALLBACK_NAME: &str = "user.name=ci-promote";
const FALLBACK_EMAIL: &str = "user.email=ci-promote@localhost";

pub struct Signing {
    _dir: TempDir,
    allowed: PathBuf,
    config: Vec<String>,
}

impl Signing {
    pub fn new(commands: &dyn Commands, git: &Git, key: &OsStr) -> io::Result<Self> {
        let dir = tempfile::Builder::new().prefix("ci-promote.").tempdir()?;
        fs::set_permissions(dir.path(), Permissions::from_mode(PRIVATE_DIR))?;
        let key_path = dir.path().join("key");
        write_private(&key_path, key)?;

        let allowed = dir.path().join("allowed");
        let public = commands.run(&Invocation::new(
            "ssh-keygen",
            &["-y", "-P", "", "-f", &key_path.to_string_lossy()],
        ));
        let signers = if public.success {
            format!("deploy namespaces=\"git\" {}\n", public.line())
        } else {
            String::new()
        };
        fs::write(&allowed, signers)?;

        let mut config = vec![
            "-c".to_owned(),
            "gpg.format=ssh".to_owned(),
            "-c".to_owned(),
            format!("user.signingKey={}", key_path.display()),
        ];
        if !git.quiet(&["config", "user.email"]).success {
            config.extend(
                ["-c", FALLBACK_NAME, "-c", FALLBACK_EMAIL]
                    .iter()
                    .map(|&arg| arg.to_owned()),
            );
        }
        Ok(Self {
            _dir: dir,
            allowed,
            config,
        })
    }

    pub fn config(&self) -> &[String] {
        &self.config
    }

    pub fn verified(&self, git: &Git, host: &str, commit: &str) -> bool {
        let allowed = format!("gpg.ssh.allowedSignersFile={}", self.allowed.display());
        let verified = git.run(
            &git.invocation(&[
                "-c",
                "gpg.format=ssh",
                "-c",
                &allowed,
                "-c",
                "gpg.openpgp.program=false",
                "-c",
                "gpg.x509.program=false",
                "verify-commit",
                commit,
            ])
            .discard_output(),
        );
        verified.success && is_promotion(git, host, commit)
    }
}

fn write_private(path: &Path, key: &OsStr) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE)
        .open(path)?;
    file.write_all(key.as_bytes())?;
    file.write_all(b"\n")
}
