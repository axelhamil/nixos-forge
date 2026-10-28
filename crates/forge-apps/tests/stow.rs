#[path = "../../forge/tests/common/mod.rs"]
mod common;

use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Output;

use tempfile::TempDir;

const FAKE_STOW: &str = r#"#!/bin/sh
echo "$*" >> "$STOW_LOG"
for package; do :; done
if [ -n "$STOW_FAIL" ] && [ "$package" = "$STOW_FAIL" ]; then
  printf '%s\n' "$STOW_REPORT" >&2
  exit 3
fi
"#;

struct Home {
    dir: TempDir,
    fail: Option<(String, String)>,
}

impl Home {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("bin")).unwrap();
        common::write_executable(&dir.path().join("bin/stow"), FAKE_STOW);
        Self { dir, fail: None }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.dir.path().join(relative)
    }

    fn text(&self, relative: &str) -> String {
        self.path(relative).display().to_string()
    }

    fn make(&self, relative: &str) -> &Self {
        fs::create_dir_all(self.path(relative)).unwrap();
        self
    }

    fn failing(mut self, package: &str, report: &str) -> Self {
        self.fail = Some((package.to_owned(), report.to_owned()));
        self
    }

    fn run(&self, args: &[&str]) -> Output {
        let (fail, report) = self.fail.clone().unwrap_or_default();
        common::sandboxed(
            Path::new(env!("CARGO_BIN_EXE_stow-dotfiles")),
            self.dir.path(),
            &self.path("bin"),
            self.dir.path(),
        )
        .env("USER", "alice")
        .env("STOW_LOG", self.path("stow.log"))
        .env("STOW_FAIL", fail)
        .env("STOW_REPORT", report)
        .args(args)
        .output()
        .unwrap()
    }

    fn stow(&self, mode: &str, extra: &[&str]) -> Output {
        let dotfiles = self.text("dotfiles");
        let target = self.text("home");
        let state = self.text("state/stow/packages");
        let mut args = vec![mode, &dotfiles, &target, &state];
        args.extend(extra);
        self.run(&args)
    }

    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.path("stow.log"))
            .unwrap_or_default()
            .lines()
            .map(|line| line.replace(&format!("{}/", self.dir.path().display()), ""))
            .collect()
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn fewer_than_four_arguments_print_the_usage() {
    let home = Home::new();

    let output = home.run(&["apply", "a", "b"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        stderr(&output),
        "usage: stow-dotfiles prune|check|apply <dotfiles> <target> <state> [--dotfiles] [package...]\n"
    );
}

#[test]
fn missing_dotfiles_are_announced_by_apply_only() {
    let home = Home::new();

    let applied = home.stow("apply", &[]);
    let pruned = home.stow("prune", &[]);

    assert!(applied.status.success() && pruned.status.success());
    assert_eq!(
        stdout(&applied),
        format!(
            "stow: no dotfiles in {} yet; clone them there, then run: systemctl restart home-manager-alice\n",
            home.text("dotfiles")
        )
    );
    assert_eq!(stdout(&pruned), "");
    assert_eq!(home.calls(), Vec::<String>::new());
}

#[test]
fn apply_links_every_top_level_directory_in_order_and_records_them() {
    let home = Home::new();
    home.make("dotfiles/zsh")
        .make("dotfiles/git")
        .make("dotfiles/.git")
        .make("dotfiles/.stow-local-ignore");
    fs::write(home.path("dotfiles/README.md"), "docs").unwrap();

    let output = home.stow("apply", &["--dotfiles"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        home.calls(),
        [
            "--no-folding --target home --dotfiles --dir dotfiles --restow git",
            "--no-folding --target home --dotfiles --dir dotfiles --restow zsh",
        ]
    );
    assert_eq!(
        fs::read_to_string(home.path("state/stow/packages")).unwrap(),
        "git\nzsh\n"
    );
}

#[test]
fn apply_links_and_records_a_package_whose_name_is_not_utf8() {
    let home = Home::new();
    let package = OsStr::from_bytes(b"pk\xff");
    fs::create_dir_all(home.path("dotfiles").join(package)).unwrap();

    let output = home.stow("apply", &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    let log = fs::read(home.path("stow.log")).unwrap();
    assert!(log.ends_with(b" --restow pk\xff\n"), "{log:?}");
    assert_eq!(
        fs::read(home.path("state/stow/packages")).unwrap(),
        b"pk\xff\n"
    );
}

#[test]
fn a_mirror_of_the_home_is_stowed_through_a_link_beside_the_state() {
    let home = Home::new();
    home.make("dotfiles/.config/tool").make("dotfiles/nvim");

    let output = home.stow("apply", &[]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        home.calls(),
        ["--no-folding --target home --dir state/stow/mirror --restow home"]
    );
    assert_eq!(
        fs::read_link(home.path("state/stow/mirror/home")).unwrap(),
        home.path("dotfiles")
    );
    assert_eq!(
        fs::read_to_string(home.path("state/stow/packages")).unwrap(),
        ".\n"
    );
}

#[test]
fn a_named_package_missing_from_the_dotfiles_is_skipped() {
    let home = Home::new();
    home.make("dotfiles/zsh").make("dotfiles/git");

    let output = home.stow("apply", &["zsh", "vim"]);

    assert!(output.status.success());
    assert_eq!(
        stderr(&output),
        format!(
            "stow: no package vim in {}, skipped\n",
            home.text("dotfiles")
        )
    );
    assert_eq!(
        home.calls(),
        ["--no-folding --target home --dir dotfiles --restow zsh"]
    );
}

#[test]
fn prune_unlinks_the_recorded_packages_no_longer_wanted() {
    let home = Home::new();
    home.make("dotfiles/zsh")
        .make("dotfiles/git")
        .make("state/stow");
    fs::write(home.path("state/stow/packages"), "zsh\ngit\ngone\n\n").unwrap();

    let output = home.stow("prune", &["zsh"]);

    assert!(output.status.success());
    assert_eq!(
        home.calls(),
        ["--no-folding --target home --dir dotfiles --delete git"]
    );
}

#[test]
fn prune_stops_with_the_code_of_a_failing_stow() {
    let home = Home::new().failing("git", "cannot unlink");
    home.make("dotfiles/git")
        .make("dotfiles/zsh")
        .make("state/stow");
    fs::write(home.path("state/stow/packages"), "git\n").unwrap();

    let output = home.stow("prune", &["zsh"]);

    assert_eq!(output.status.code(), Some(3));
    assert_eq!(stderr(&output), "cannot unlink\n");
}

#[test]
fn check_names_the_files_home_manager_also_writes() {
    let home = Home::new().failing(
        "git",
        "WARNING! stowing git would cause conflicts:\n  * existing target is neither a link nor a directory: .config/git/config\nAll operations aborted.",
    );
    home.make("dotfiles/git").make("dotfiles/zsh");

    let output = home.stow("check", &[]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stderr(&output),
        "stow: git has files that home-manager also writes:\n\
         existing target is neither a link nor a directory: .config/git/config\n\
         stow: turn off the module that writes them in your home config (for instance programs.git.enable = false), or remove them from your dotfiles\n"
    );
    assert_eq!(
        home.calls(),
        ["--no-folding --target home --dir dotfiles --simulate git"]
    );
}

#[test]
fn check_prints_any_other_failure_as_stow_reported_it() {
    let home = Home::new().failing("zsh", "stow: ERROR: something else\n");
    home.make("dotfiles/zsh");

    let output = home.stow("check", &[]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stderr(&output), "stow: ERROR: something else\n");
}

#[test]
fn apply_reports_a_package_it_could_not_link_and_still_records_the_list() {
    let home = Home::new().failing("git", "conflict");
    home.make("dotfiles/git");

    let output = home.stow("apply", &[]);

    assert!(output.status.success());
    assert_eq!(
        stderr(&output),
        "conflict\nstow: git not linked: move the files listed above out of the way, then run: systemctl restart home-manager-alice\n"
    );
    assert_eq!(
        fs::read_to_string(home.path("state/stow/packages")).unwrap(),
        "git\n"
    );
}

#[test]
fn apply_with_no_package_leaves_an_empty_state() {
    let home = Home::new();
    home.make("dotfiles").make("state/stow");
    fs::write(home.path("state/stow/packages"), "old\n").unwrap();

    let output = home.stow("apply", &[]);

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(home.path("state/stow/packages")).unwrap(),
        ""
    );
}
