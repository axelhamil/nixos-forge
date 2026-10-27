mod common;

use std::env;
use std::fs;
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{find_program, sandboxed, write_executable};
use tempfile::TempDir;

const BRANCH: &str = "chore/sync-upstream-v0.2.0";

struct World {
    dir: TempDir,
    starter: String,
    v020: String,
}

struct Run {
    status: i32,
    out: String,
}

impl World {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let world = Self {
            starter: format!("file://{}/starter", dir.path().display()),
            dir,
            v020: String::new(),
        };
        fs::create_dir_all(world.home()).unwrap();
        fs::create_dir_all(world.bin()).unwrap();
        symlink(find_program("git"), world.bin().join("git")).unwrap();
        fs::write(
            world.home().join(".gitconfig"),
            "[init]\n\tdefaultBranch = main\n[advice]\n\tdetachedHead = false\n[user]\n\tname = test\n\temail = test@example.com\n",
        )
        .unwrap();
        world.with_starter()
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn home(&self) -> PathBuf {
        self.root().join("home")
    }

    fn bin(&self) -> PathBuf {
        self.root().join("bin")
    }

    fn command(&self, program: &Path, dir: &Path) -> Command {
        sandboxed(program, dir, &self.bin(), &self.home())
    }

    fn git_in(&self, dir: &Path, args: &[&str]) -> (bool, String) {
        let output = self
            .command(&self.bin().join("git"), dir)
            .args(args)
            .output()
            .unwrap();
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_owned(),
        )
    }

    fn git(&self, dir: &Path, args: &[&str]) -> String {
        let (success, stdout) = self.git_in(dir, args);
        assert!(success, "git {args:?} failed in {}", dir.display());
        stdout
    }

    fn with_starter(mut self) -> Self {
        let starter = self.root().join("starter");
        fs::create_dir_all(&starter).unwrap();
        let git = |args: &[&str]| self.git(&starter, args);
        git(&["init", "-q"]);
        fs::write(starter.join("CHANGELOG.md"), "# Changelog\n").unwrap();
        fs::write(starter.join("x"), "x\n").unwrap();
        git(&["add", "CHANGELOG.md", "x"]);
        git(&["commit", "-q", "-m", "feat: first release"]);
        git(&["tag", "v0.1.0"]);
        fs::write(starter.join("x"), "y\n").unwrap();
        git(&["commit", "-q", "-am", "feat!: rename x"]);
        fs::write(
            starter.join("CHANGELOG.md"),
            "# Changelog\n\n## 0.2.0\n\n* rename x to y\n",
        )
        .unwrap();
        git(&["commit", "-q", "-am", "chore: release 0.2.0"]);
        git(&["tag", "v0.2.0"]);
        fs::write(starter.join("x"), "nightly\n").unwrap();
        git(&["commit", "-q", "-am", "chore: nightly build"]);
        git(&["tag", "zzz"]);
        git(&["tag", "-a", "-m", "nightly", "nightly"]);
        git(&["switch", "-q", "--detach", "v0.2.0"]);
        self.v020 = git(&["rev-parse", "v0.2.0"]);
        self
    }

    fn origin(&self, name: &str) -> PathBuf {
        self.root().join(format!("{name}-origin.git"))
    }

    fn bare_origin(&self, name: &str) -> PathBuf {
        let fork = self.root().join(name);
        fs::create_dir_all(&fork).unwrap();
        self.git(
            self.root(),
            &[
                "init",
                "-q",
                "--bare",
                &self.origin(name).display().to_string(),
            ],
        );
        self.git(&fork, &["init", "-q"]);
        fork
    }

    fn fork(&self, name: &str) -> PathBuf {
        let fork = self.bare_origin(name);
        let git = |args: &[&str]| self.git(&fork, args);
        git(&[
            "fetch",
            "-q",
            "--no-tags",
            &self.starter,
            "refs/tags/v0.1.0:refs/tags/v0.1.0",
        ]);
        git(&["checkout", "-q", "-B", "main", "v0.1.0"]);
        self.add_origin(name, &fork);
        fork
    }

    fn add_origin(&self, name: &str, fork: &Path) {
        let url = format!("file://{}", self.origin(name).display());
        self.git(fork, &["remote", "add", "origin", &url]);
        self.git(fork, &["push", "-q", "origin", "main"]);
    }

    fn upstream(&self, fork: &Path, args: &[&str], stdin: &str, ci: bool) -> Run {
        let mut command = self.command(Path::new(env!("CARGO_BIN_EXE_forge")), fork);
        command
            .arg("upstream")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if ci {
            command.env("GITHUB_ACTIONS", "true");
        }
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        Run {
            status: output.status.code().unwrap(),
            out: format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
        }
    }

    fn sync(&self, fork: &Path) -> Run {
        self.upstream(fork, &["--yes", &self.starter], "", false)
    }

    fn on_origin(&self, name: &str) -> bool {
        let git_dir = format!("--git-dir={}", self.origin(name).display());
        self.git_in(
            self.root(),
            &[
                &git_dir,
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{BRANCH}"),
            ],
        )
        .0
    }

    fn has_branch(&self, fork: &Path) -> bool {
        self.git_in(
            fork,
            &[
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/{BRANCH}"),
            ],
        )
        .0
    }

    fn assert_clean_after(&self, what: &str, run: &Run, fork: &Path, name: &str) {
        assert_eq!(run.status, 1, "{what}: {}", run.out);
        assert_eq!(self.git(fork, &["status", "--porcelain"]), "", "{what}");
        assert!(
            !self
                .git_in(fork, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
                .0,
            "{what} left a merge in progress"
        );
        assert!(!self.has_branch(fork), "{what} left {BRANCH} behind");
        assert!(!self.on_origin(name), "{what} pushed {BRANCH}");
    }
}

fn lines(text: &str) -> Vec<&str> {
    text.lines().collect()
}

#[test]
fn a_sync_merges_the_latest_release_from_origin_main_and_prints_the_pull_request_command() {
    let world = World::new();
    let fork = world.fork("synced");
    world.git(&fork, &["switch", "-q", "-c", "develop"]);
    fs::write(fork.join("mine"), "mine\n").unwrap();
    world.git(&fork, &["add", "mine"]);
    world.git(&fork, &["commit", "-q", "-m", "feat: not released yet"]);
    let unreleased = world.git(&fork, &["rev-parse", "HEAD"]);

    let run = world.sync(&fork);

    assert_eq!(run.status, 0, "{}", run.out);
    let out = lines(&run.out);
    let breaking = out
        .iter()
        .position(|line| line.starts_with("Breaking changes"))
        .unwrap();
    assert_eq!(out[breaking + 1], "feat!: rename x", "{}", run.out);
    assert!(run.out.contains("* rename x to y"), "{}", run.out);
    assert!(
        !run.out.contains("nightly") && !run.out.contains("zzz"),
        "{}",
        run.out
    );
    assert!(world.on_origin("synced"));
    assert_eq!(
        world.git(&fork, &["rev-parse", &format!("{BRANCH}^2")]),
        world.v020
    );
    assert_eq!(
        world.git(&fork, &["rev-parse", &format!("{BRANCH}^1")]),
        world.git(&fork, &["rev-parse", "main"])
    );
    assert!(
        !world
            .git_in(&fork, &["merge-base", "--is-ancestor", &unreleased, BRANCH])
            .0
    );
    let root_name = world.root().file_name().unwrap().to_string_lossy();
    assert!(
        run.out.contains(&format!(
            "gh pr create --repo {root_name}/synced-origin --base main"
        )),
        "{}",
        run.out
    );

    let body_path = format!(
        "{}/upstream-v0.2.0.md",
        world.git(&fork, &["rev-parse", "--absolute-git-dir"])
    );
    let body = fs::read_to_string(body_path).unwrap();
    let body_lines = lines(&body);
    assert!(
        body_lines[0].starts_with(&format!("Merges {} into this fork. ", world.starter)),
        "{body}"
    );
    let upstream_commits = world.git(&fork, &["rev-list", "main..v0.2.0"]);
    assert_eq!(upstream_commits.lines().count(), 2);
    for commit in upstream_commits.lines() {
        let oneline = world.git(&fork, &["log", "-1", "--oneline", "--no-decorate", commit]);
        assert!(body_lines.contains(&oneline.as_str()), "{body}");
    }
    assert!(!body.contains("not released yet"), "{body}");
    assert!(body_lines.contains(&"feat!: rename x"), "{body}");

    world.git(&fork, &["switch", "-q", "main"]);
    world.git(&fork, &["merge", "-q", "--ff-only", BRANCH]);
    world.git(&fork, &["push", "-q", "origin", "main"]);
    world.git(&fork, &["switch", "-q", "develop"]);

    let again = world.sync(&fork);

    assert_eq!(again.status, 0, "{}", again.out);
    assert!(
        again.out.contains("Already up to date with v0.2.0"),
        "{}",
        again.out
    );
}

#[test]
fn a_conflict_from_a_branch_or_a_detached_head_leaves_the_fork_where_it_was() {
    let world = World::new();
    let fork = world.fork("conflicting");
    fs::write(fork.join("x"), "z\n").unwrap();
    world.git(&fork, &["commit", "-q", "-am", "feat: my own x"]);
    world.git(&fork, &["push", "-q", "origin", "main"]);

    let run = world.sync(&fork);

    world.assert_clean_after("conflict", &run, &fork, "conflicting");
    assert!(run.out.contains(" in: x."), "{}", run.out);
    assert!(
        run.out
            .contains(&format!("git switch -c {BRANCH} origin/main")),
        "{}",
        run.out
    );
    assert_eq!(world.git(&fork, &["branch", "--show-current"]), "main");

    world.git(&fork, &["switch", "-q", "--detach", "HEAD~1"]);
    let detached = world.git(&fork, &["rev-parse", "HEAD"]);

    let run = world.sync(&fork);

    world.assert_clean_after("conflict from a detached HEAD", &run, &fork, "conflicting");
    assert!(run.out.contains(" in: x."), "{}", run.out);
    assert!(!world.git_in(&fork, &["symbolic-ref", "-q", "HEAD"]).0);
    assert_eq!(world.git(&fork, &["rev-parse", "HEAD"]), detached);
}

#[test]
fn unrelated_histories_stop_with_the_reason_git_gives() {
    let world = World::new();
    let fork = world.bare_origin("unrelated");
    fs::write(fork.join("own"), "own\n").unwrap();
    world.git(&fork, &["add", "own"]);
    world.git(&fork, &["commit", "-q", "-m", "feat: copied, not forked"]);
    world.add_origin("unrelated", &fork);

    let run = world.sync(&fork);

    world.assert_clean_after("unrelated histories", &run, &fork, "unrelated");
    assert!(run.out.contains("unrelated histories"), "{}", run.out);
    assert_eq!(world.git(&fork, &["branch", "--show-current"]), "main");
}

#[test]
fn declining_the_prompt_for_the_upstream_remote_changes_nothing() {
    let world = World::new();
    let fork = world.fork("declined");
    world.git(&fork, &["remote", "add", "upstream", &world.starter]);

    let run = world.upstream(&fork, &[], "n\n", false);

    assert_eq!(run.status, 0, "{}", run.out);
    assert!(
        run.out.contains(&format!("Prepare branch {BRANCH}? [y/N]")),
        "{}",
        run.out
    );
    assert!(run.out.contains("Nothing changed."), "{}", run.out);
    assert!(!world.has_branch(&fork));
    assert!(!world.on_origin("declined"));
}

#[test]
fn a_branch_already_on_origin_waits_for_its_review() {
    let world = World::new();
    let fork = world.fork("waiting");
    world.git(
        &fork,
        &["push", "-q", "origin", &format!("main:refs/heads/{BRANCH}")],
    );

    let run = world.sync(&fork);

    assert_eq!(run.status, 0, "{}", run.out);
    assert!(
        run.out.contains(&format!(
            "{BRANCH} is already on origin: its pull request is waiting for your review."
        )),
        "{}",
        run.out
    );
}

#[test]
fn a_local_branch_or_a_dirty_tree_is_refused_before_anything_changes() {
    let world = World::new();
    let fork = world.fork("refused");
    world.git(&fork, &["branch", BRANCH]);

    let run = world.sync(&fork);

    assert_eq!(run.status, 1, "{}", run.out);
    assert!(
        run.out.contains(&format!(
            "forge upstream: {BRANCH} already exists here: push it, or delete it with git branch -D {BRANCH} to prepare it again"
        )),
        "{}",
        run.out
    );

    world.git(&fork, &["branch", "-D", BRANCH]);
    fs::write(fork.join("x"), "dirty\n").unwrap();

    let run = world.sync(&fork);

    assert_eq!(run.status, 1, "{}", run.out);
    assert!(
        run.out
            .contains("forge upstream: commit or stash your changes first"),
        "{}",
        run.out
    );
    assert!(!world.has_branch(&fork));
}

#[test]
fn in_github_actions_the_merge_is_made_by_the_bot_and_gh_opens_the_pull_request() {
    let world = World::new();
    let fork = world.fork("bot");
    let seen = world.root().join("gh-args");
    let gh = world.bin().join("gh");
    write_executable(
        &gh,
        &format!(
            "#!/bin/sh\n[ \"$1\" = auth ] && exit 0\nprintf '%s\\n' \"$@\" > {}\n",
            seen.display()
        ),
    );

    let run = world.upstream(&fork, &["--yes", &world.starter], "", true);

    assert_eq!(run.status, 0, "{}", run.out);
    assert_eq!(
        world.git(&fork, &["log", "-1", "--format=%an <%ae>", BRANCH]),
        "github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>"
    );
    let root_name = world.root().file_name().unwrap().to_string_lossy();
    let body = format!(
        "{}/upstream-v0.2.0.md",
        world.git(&fork, &["rev-parse", "--absolute-git-dir"])
    );
    assert_eq!(
        fs::read_to_string(seen).unwrap(),
        format!(
            "pr\ncreate\n--repo\n{root_name}/bot-origin\n--base\nmain\n--head\n{BRANCH}\n--title\nchore: sync with upstream v0.2.0\n--body-file\n{body}\n"
        )
    );
}

#[test]
fn an_unknown_option_is_a_usage_error() {
    let world = World::new();

    let run = world.upstream(world.root(), &["--force"], "", false);

    assert_eq!(run.status, 2);
    assert_eq!(
        run.out,
        "forge upstream: unknown option --force (see --help)\n"
    );
}
