mod common;

use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Output;

const FAKE_NIX: &str = r#"
host_of() {
  sed -E 's/.*nixosConfigurations\.([^.]+)\..*/\1/' <<<"$1"
}

case "$1" in
eval)
  [ "$*" = 'eval --impure --json .#ci.hosts --apply hosts: hosts.${builtins.currentSystem}' ] || {
    echo "unexpected nix call: $*" >&2
    exit 1
  }
  echo '["same", "differs"]'
  ;;
build)
  installable=${*: -1}
  host=$(host_of "$installable")
  path=$STORE/$host-system

  if [ "$host" = broken ]; then
    echo "error: builder for '$host' failed" >&2
    exit 1
  fi

  if [[ " $* " != *" --rebuild "* ]]; then
    mkdir -p "$path"
    echo "built at 1" >"$path/version"
    echo "$path"
    exit 0
  fi

  case "$host" in
  differs)
    mkdir -p "$path.check"
    echo "built at 2" >"$path.check/version"
    echo "error: derivation '$host.drv' may not be deterministic: output '$path' differs from '$path.check'" >&2
    exit 1
    ;;
  unfinished)
    echo "error: the rebuild of '$host' was killed" >&2
    exit 1
    ;;
  esac
  ;;
*)
  echo "unexpected nix call: $*" >&2
  exit 1
  ;;
esac
"#;

const FAKE_DIFFOSCOPE: &str = r#"
[ "$1" = --text ] && [ "$3" = --exclude-directory-metadata=yes ] || exit 2
{
  diff -ru "$4" "$5"
  seq 300
} >"$2"
echo "diffoscope wrote $2"
exit 1
"#;

struct Sandbox {
    dir: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let sandbox = Self {
            dir: tempfile::tempdir().unwrap(),
        };
        fs::create_dir_all(sandbox.bin()).unwrap();
        fs::create_dir_all(sandbox.store()).unwrap();
        let shebang = format!("#!{}\n", common::find_program("bash").display());
        common::write_executable(&sandbox.bin().join("nix"), &(shebang.clone() + FAKE_NIX));
        common::write_executable(
            &sandbox.bin().join("diffoscope"),
            &(shebang + FAKE_DIFFOSCOPE),
        );
        sandbox
    }

    fn bin(&self) -> PathBuf {
        self.dir.path().join("bin")
    }

    fn store(&self) -> PathBuf {
        self.dir.path().join("store")
    }

    fn run(&self, hosts: &[&str]) -> Output {
        let mut path = OsString::from(self.bin());
        path.push(":");
        path.push(env::var_os("PATH").unwrap_or_default());
        common::sandboxed(
            Path::new(env!("CARGO_BIN_EXE_forge-ci")),
            self.dir.path(),
            &self.bin(),
            self.dir.path(),
        )
        .env("PATH", path)
        .env("STORE", self.store())
        .arg("reproducibility")
        .args(hosts)
        .output()
        .unwrap()
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn has_line(markdown: &str, line: &str) -> bool {
    markdown.lines().any(|candidate| candidate == line)
}

#[test]
fn an_identical_and_a_differing_rebuild_are_reported_with_the_diffoscope_summary() {
    let sandbox = Sandbox::new();
    fs::create_dir_all(sandbox.store().join("same-system.check/stale")).unwrap();

    let output = sandbox.run(&[]);

    let summary = text(&output.stdout);
    assert!(output.status.success(), "{output:?}");
    assert!(summary.starts_with(
        "## Reproducibility\n\nEach system is built, then built again with `nix build --rebuild` and compared. A difference is expected now and then and fails nothing.\n\n"
    ));
    assert!(has_line(&summary, "- `same`: the rebuild is identical."));
    assert!(has_line(
        &summary,
        "- `differs`: the rebuild differs from the first build."
    ));
    assert!(has_line(
        &summary,
        "<summary>diffoscope of differs, first 200 lines</summary>"
    ));
    assert!(summary.contains("-built at 1"), "{summary}");
    assert!(summary.contains("+built at 2"), "{summary}");
    assert!(!sandbox.store().join("same-system.check").exists());

    let fences: Vec<usize> = summary
        .lines()
        .enumerate()
        .filter(|(_, line)| *line == "```")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(fences.len(), 2, "{summary}");
    assert_eq!(fences[1] - fences[0] - 1, 200);
    assert!(summary.ends_with("```\n</details>\n\n"), "{summary}");
    assert!(!summary.contains("diffoscope wrote"));
}

#[test]
fn a_system_that_does_not_build_fails_the_run_after_the_others() {
    let sandbox = Sandbox::new();

    let output = sandbox.run(&["same", "broken"]);

    assert_eq!(output.status.code(), Some(1));
    let summary = text(&output.stdout);
    assert!(has_line(
        &summary,
        "- `broken`: the system did not build, see the job log."
    ));
    assert!(has_line(&summary, "- `same`: the rebuild is identical."));
    assert!(text(&output.stderr).contains("builder for 'broken' failed"));
}

#[test]
fn an_unfinished_rebuild_or_a_check_that_cannot_be_removed_fails_the_run() {
    let sandbox = Sandbox::new();
    let locked = sandbox.store().join("stuck-system.check/locked");
    fs::create_dir_all(&locked).unwrap();
    fs::write(locked.join("file"), "").unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();

    let output = sandbox.run(&["unfinished", "stuck", "same"]);

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let summary = text(&output.stdout);
    assert!(has_line(
        &summary,
        "- `unfinished`: the rebuild did not finish, see the job log."
    ));
    assert!(has_line(
        &summary,
        &format!(
            "- `stuck`: `{}/stuck-system.check` is left from an earlier run and cannot be removed, so a difference could not be told apart; remove it as root and run again.",
            sandbox.store().display()
        )
    ));
    assert!(has_line(&summary, "- `same`: the rebuild is identical."));
    assert!(text(&output.stderr).contains("the rebuild of 'unfinished' was killed"));
}

#[test]
fn a_flag_is_a_usage_error_and_help_prints_the_usage() {
    let sandbox = Sandbox::new();

    let flag = sandbox.run(&["--all"]);
    assert_eq!(flag.status.code(), Some(2));
    assert_eq!(text(&flag.stderr), "usage: reproducibility [<host>...]\n");

    let help = sandbox.run(&["-h"]);
    assert_eq!(help.status.code(), Some(0));
    assert_eq!(text(&help.stdout), "usage: reproducibility [<host>...]\n");
}
