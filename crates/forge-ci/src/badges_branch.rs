use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use forge::system::{Commands, Git, Invocation};

use crate::results::json_files;

pub const BRANCH: &str = "badges";

pub const GIT_TIMEOUT: Duration = Duration::from_mins(45);

const ATTEMPTS: u32 = 3;
const BOT_NAME: &str = "user.name=github-actions[bot]";
const BOT_EMAIL: &str = "user.email=41898282+github-actions[bot]@users.noreply.github.com";
const READ_FAILED: u8 = 1;
const READ_MISSING: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    #[error("the {BRANCH} branch has no {0}")]
    NoFile(String),
    #[error("origin has no {BRANCH} branch yet")]
    NoBranch,
    #[error(
        "the {BRANCH} branch of origin could not be fetched; a private repository needs a checkout that keeps its credentials"
    )]
    Fetch,
}

impl ReadError {
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::NoFile(_) | Self::NoBranch => READ_MISSING,
            Self::Fetch => READ_FAILED,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Published {
    Unchanged,
    Pushed(Vec<String>),
}

impl Published {
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            Self::Unchanged => "the badges did not change".to_owned(),
            Self::Pushed(names) => format!("published {}", names.join(" ")),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("{0} does not exist")]
    NoDir(String),
    #[error("{0} holds no badge")]
    NoBadges(String),
    #[error("the {BRANCH} branch of origin could not be fetched")]
    Fetch,
    #[error("{0} failed")]
    Git(String),
    #[error("cannot {action}: {error}")]
    Io { action: String, error: io::Error },
    #[error("the badges could not be published")]
    Rejected,
}

enum Fetched {
    Branch,
    NoBranch,
    Failed,
}

fn fetch_branch(git: &Git) -> Fetched {
    let head = format!("refs/heads/{BRANCH}");
    let listed = git.run(
        &git.invocation(&["ls-remote", "--exit-code", "origin", &head])
            .timeout(GIT_TIMEOUT)
            .discard_output(),
    );
    match listed.code {
        Some(0) => {}
        Some(2) => return Fetched::NoBranch,
        _ => return Fetched::Failed,
    }

    let fetched = git.run(
        &git.invocation(&["fetch", "-q", "--depth", "1", "origin", BRANCH])
            .timeout(GIT_TIMEOUT),
    );
    if fetched.success {
        Fetched::Branch
    } else {
        Fetched::Failed
    }
}

pub fn read(commands: &dyn Commands, repo: &str, file: &str) -> Result<String, ReadError> {
    let git = Git::new(commands, repo);
    match fetch_branch(&git) {
        Fetched::Branch => {}
        Fetched::NoBranch => return Err(ReadError::NoBranch),
        Fetched::Failed => return Err(ReadError::Fetch),
    }

    let shown = git.quiet(&["show", &format!("FETCH_HEAD:{file}")]);
    if !shown.success {
        return Err(ReadError::NoFile(file.to_owned()));
    }
    Ok(shown.stdout)
}

struct Worktree<'a> {
    repo: &'a Git<'a>,
    path: String,
}

impl Worktree<'_> {
    fn remove(&self) {
        let _ = self.repo.run(
            &self
                .repo
                .invocation(&["worktree", "remove", "--force", &self.path]),
        );
    }
}

impl Drop for Worktree<'_> {
    fn drop(&mut self) {
        self.remove();
    }
}

fn succeed(git: &Git, invocation: &Invocation) -> Result<(), PublishError> {
    if git.run(invocation).success {
        return Ok(());
    }
    Err(PublishError::Git(invocation.command_line()))
}

fn io_error(action: &str) -> impl FnOnce(io::Error) -> PublishError {
    let action = action.to_owned();
    move |error| PublishError::Io { action, error }
}

pub fn publish(
    commands: &dyn Commands,
    repo: &str,
    dir: &Path,
    err: &mut dyn Write,
) -> Result<Published, PublishError> {
    if !dir.is_dir() {
        return Err(PublishError::NoDir(dir.display().to_string()));
    }

    let dir = fs::canonicalize(dir).map_err(io_error("resolve the badges dir"))?;
    let badges = json_files(&dir);
    if badges.is_empty() {
        return Err(PublishError::NoBadges(dir.display().to_string()));
    }

    let names: Vec<String> = badges
        .iter()
        .filter_map(|badge| Some(badge.file_name()?.to_string_lossy().into_owned()))
        .collect();
    let work = tempfile::tempdir().map_err(io_error("create a temporary directory"))?;
    let checkout = work.path().join("badges");
    let repo = Git::new(commands, repo);
    let worktree = Worktree {
        repo: &repo,
        path: checkout.display().to_string(),
    };
    let inside = Git::new(commands, &worktree.path);

    for attempt in 1..=ATTEMPTS {
        worktree.remove();
        let add = |start: &str| {
            repo.invocation(&["worktree", "add", "-q", "--detach", &worktree.path, start])
                .loud()
        };
        match fetch_branch(&repo) {
            Fetched::Branch => succeed(&repo, &add("FETCH_HEAD"))?,
            Fetched::NoBranch => {
                succeed(&repo, &add("HEAD"))?;
                succeed(
                    &inside,
                    &inside
                        .invocation(&["checkout", "-q", "--orphan", BRANCH])
                        .loud(),
                )?;
                succeed(
                    &inside,
                    &inside.invocation(&["rm", "-q", "-r", "-f", "."]).loud(),
                )?;
            }
            Fetched::Failed => return Err(PublishError::Fetch),
        }

        copy(&badges, &checkout)?;
        succeed(&inside, &inside.invocation(&["add", "-A"]).loud())?;
        if inside.quiet(&["diff", "--cached", "--quiet"]).success {
            return Ok(Published::Unchanged);
        }

        succeed(
            &inside,
            &inside
                .invocation(&[
                    "-c",
                    BOT_NAME,
                    "-c",
                    BOT_EMAIL,
                    "commit",
                    "-q",
                    "-m",
                    "chore: update the badges",
                ])
                .loud(),
        )?;
        let target = format!("HEAD:refs/heads/{BRANCH}");
        let pushed = inside.run(
            &inside
                .invocation(&["push", "-q", "origin", &target])
                .timeout(GIT_TIMEOUT)
                .loud(),
        );
        if pushed.success {
            return Ok(Published::Pushed(names));
        }

        let _ = writeln!(
            err,
            "ci-badges: push {attempt} was rejected, retrying on the new {BRANCH} branch"
        );
    }

    Err(PublishError::Rejected)
}

fn copy(badges: &[PathBuf], checkout: &Path) -> Result<(), PublishError> {
    for badge in badges {
        let Some(name) = badge.file_name() else {
            continue;
        };
        fs::copy(badge, checkout.join(name)).map_err(io_error("copy the badges"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use forge::system::{Answer, Invocation};

    use super::*;

    struct Remote {
        listed: i32,
        fetch: bool,
        show: Answer,
        calls: RefCell<Vec<String>>,
    }

    impl Remote {
        fn new(listed: i32) -> Self {
            Self {
                listed,
                fetch: true,
                show: Answer::ok("{\"tree\":null}\n"),
                calls: RefCell::new(Vec::new()),
            }
        }
    }

    impl Commands for Remote {
        fn run(&self, invocation: &Invocation) -> Answer {
            let line = invocation.command_line();
            self.calls.borrow_mut().push(line.clone());
            match line.as_str() {
                "git -C . ls-remote --exit-code origin refs/heads/badges" => {
                    Answer::exited(self.listed)
                }
                "git -C . fetch -q --depth 1 origin badges" if self.fetch => Answer::ok(""),
                "git -C . show FETCH_HEAD:flaky.json" => self.show.clone(),
                _ => Answer::failed("fatal: unexpected"),
            }
        }
    }

    #[test]
    fn a_file_of_the_fetched_branch_is_read() {
        let remote = Remote::new(0);
        assert_eq!(
            read(&remote, ".", "flaky.json").unwrap(),
            "{\"tree\":null}\n"
        );
        assert_eq!(
            *remote.calls.borrow(),
            [
                "git -C . ls-remote --exit-code origin refs/heads/badges",
                "git -C . fetch -q --depth 1 origin badges",
                "git -C . show FETCH_HEAD:flaky.json",
            ]
        );
    }

    #[test]
    fn a_missing_file_or_branch_exits_3() {
        let remote = Remote {
            show: Answer::failed("fatal: path 'flaky.json' does not exist"),
            ..Remote::new(0)
        };
        let error = read(&remote, ".", "flaky.json").unwrap_err();
        assert_eq!(error, ReadError::NoFile("flaky.json".to_owned()));
        assert_eq!(error.exit_code(), 3);
        assert_eq!(error.to_string(), "the badges branch has no flaky.json");

        let error = read(&Remote::new(2), ".", "flaky.json").unwrap_err();
        assert_eq!(error, ReadError::NoBranch);
        assert_eq!(error.exit_code(), 3);
        assert_eq!(error.to_string(), "origin has no badges branch yet");
    }

    #[test]
    fn a_remote_that_cannot_be_listed_or_fetched_exits_1() {
        for remote in [
            Remote::new(128),
            Remote {
                fetch: false,
                ..Remote::new(0)
            },
        ] {
            let error = read(&remote, ".", "flaky.json").unwrap_err();
            assert_eq!(error, ReadError::Fetch);
            assert_eq!(error.exit_code(), 1);
        }
    }

    #[test]
    fn publishing_a_missing_dir_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("badges");
        let error = publish(&Remote::new(0), ".", &missing, &mut Vec::new()).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("{} does not exist", missing.display())
        );
    }

    #[test]
    fn the_published_message_lists_the_badges() {
        assert_eq!(
            Published::Pushed(vec!["checks.json".to_owned(), "hosts.json".to_owned()]).message(),
            "published checks.json hosts.json"
        );
        assert_eq!(Published::Unchanged.message(), "the badges did not change");
    }

    fn git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn publish_creates_the_branch_then_updates_it_and_keeps_the_other_badges() {
        let root = tempfile::tempdir().unwrap();
        let origin = root.path().join("origin.git");
        let clone = root.path().join("clone");
        let out = root.path().join("out");
        fs::create_dir_all(&origin).unwrap();
        fs::create_dir_all(&out).unwrap();
        git(&origin, &["init", "-q", "--bare"]);
        git(
            root.path(),
            &["clone", "-q", origin.to_str().unwrap(), "clone"],
        );
        fs::write(clone.join("README.md"), "flake\n").unwrap();
        git(&clone, &["add", "README.md"]);
        git(
            &clone,
            &[
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-q",
                "-m",
                "init",
            ],
        );
        let repo = clone.to_str().unwrap();
        let machine = forge::system::Machine;

        fs::write(out.join("checks.json"), "{}\n").unwrap();
        fs::write(out.join("reproducibility.json"), "{}\n").unwrap();
        let first = publish(&machine, repo, &out, &mut Vec::new()).unwrap();
        assert_eq!(
            first,
            Published::Pushed(vec![
                "checks.json".to_owned(),
                "reproducibility.json".to_owned()
            ])
        );

        fs::remove_file(out.join("reproducibility.json")).unwrap();
        assert_eq!(
            publish(&machine, repo, &out, &mut Vec::new()).unwrap(),
            Published::Unchanged
        );

        fs::write(out.join("checks.json"), "{\"message\":\"1/1 passing\"}\n").unwrap();
        publish(&machine, repo, &out, &mut Vec::new()).unwrap();
        assert_eq!(
            read(&machine, repo, "reproducibility.json").unwrap(),
            "{}\n"
        );
        assert_eq!(
            read(&machine, repo, "checks.json").unwrap(),
            "{\"message\":\"1/1 passing\"}\n"
        );
        assert_eq!(
            read(&machine, repo, "hosts.json").unwrap_err().exit_code(),
            3
        );
    }
}
