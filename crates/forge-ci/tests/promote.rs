mod common;

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use common::{find_program, sandboxed, write_executable};
use serde_json::{Value, json};
use tempfile::TempDir;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/runs");
const SKIPPED: &str = r#"[{"check":"example-toplevel","system":"x86_64-linux","reason":"cached"},{"check":"example-pi-sd-image","system":"aarch64-linux","reason":"compiles a kernel"}]"#;
const UNSIGNED: &str = "unsigned: DEPLOY_SIGNING_KEY is not set, so deploy branches point to main's own commits, which hosts with update.signers refuse";
const CANNOT_SIGN: &str = "deploy/example refused: DEPLOY_SIGNING_KEY cannot sign, it must hold an OpenSSH private key without a passphrase";
const IDENTITY: [(&str, &str); 4] = [
    ("GIT_AUTHOR_NAME", "test"),
    ("GIT_AUTHOR_EMAIL", "test@example.com"),
    ("GIT_COMMITTER_NAME", "test"),
    ("GIT_COMMITTER_EMAIL", "test@example.com"),
];

struct Forge {
    dir: TempDir,
    home: PathBuf,
    bin: PathBuf,
    work: PathBuf,
    tmp: PathBuf,
    log: PathBuf,
    origin: PathBuf,
}

struct Promoted {
    code: i32,
    output: String,
}

impl Promoted {
    fn has_line(&self, line: &str) -> bool {
        self.output.lines().any(|candidate| candidate == line)
    }

    fn assert_line(&self, line: &str) {
        assert!(self.has_line(line), "no line {line:?} in:\n{}", self.output);
    }
}

fn planned() -> Vec<Value> {
    let text = fs::read_to_string(Path::new(FIXTURES).join("planned.json")).unwrap();
    let planned: Vec<Value> = serde_json::from_str(&text).unwrap();
    planned
        .into_iter()
        .filter(|job| job["check"] != "example-toplevel" && job["check"] != "example-pi-sd-image")
        .collect()
}

fn short(sha: &str) -> &str {
    &sha[..7]
}

impl Forge {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let forge = Self {
            home: path.join("home"),
            bin: path.join("bin"),
            work: path.join("work"),
            tmp: path.join("tmp"),
            log: path.join("argv.log"),
            origin: path.join("origin.git"),
            dir,
        };
        for made in [&forge.home, &forge.bin, &forge.work, &forge.tmp] {
            fs::create_dir_all(made).unwrap();
        }
        forge.wrap("git");
        forge.wrap("ssh-keygen");
        symlink(
            find_program("ssh-keygen"),
            forge.bin.join("real-ssh-keygen"),
        )
        .unwrap();
        forge.git_in(&path, &["config", "--global", "init.defaultBranch", "main"]);
        forge.git_in(
            &path,
            &["config", "--global", "advice.detachedHead", "false"],
        );
        forge.git_in(&path, &["init", "-q", "--bare", "origin.git"]);
        forge.git_in(&path, &["init", "-q", "work"]);
        forge.git(&["remote", "add", "origin", "../origin.git"]);
        forge
    }

    fn wrap(&self, program: &str) {
        let stat = find_program("stat");
        let real = find_program(program);
        let log = self.log.display();
        write_executable(
            &self.bin.join(program),
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"{program} $*\" >>'{log}'\nfor arg; do\n  case $arg in\n    /*/key) '{stat}' -c '%a %n' \"$arg\" \"${{arg%/*}}\" >>'{log}' ;;\n  esac\ndone\nexec '{real}' \"$@\"\n",
                stat = stat.display(),
                real = real.display(),
            ),
        );
    }

    fn command(&self, program: &Path, dir: &Path) -> Command {
        let mut command = sandboxed(program, dir, &self.bin, &self.home);
        command.env("TMPDIR", &self.tmp).stdin(Stdio::null());
        command
    }

    fn git_in(&self, dir: &Path, args: &[&str]) -> String {
        let output = self
            .command(&self.bin.join("git"), dir)
            .envs(IDENTITY)
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.work, args)
    }

    fn commit(&self, text: &str) -> String {
        fs::write(self.work.join("file"), format!("{text}\n")).unwrap();
        self.git(&["add", "file"]);
        self.git(&["commit", "-q", "-m", text]);
        self.git(&["rev-parse", "HEAD"])
    }

    fn deploy(&self, host: &str) -> Option<String> {
        let output = self
            .command(&self.bin.join("git"), &self.origin)
            .args([
                "rev-parse",
                "--verify",
                "-q",
                &format!("refs/heads/deploy/{host}"),
            ])
            .output()
            .unwrap();
        let sha = String::from_utf8(output.stdout).unwrap().trim().to_owned();
        Some(sha).filter(|sha| !sha.is_empty())
    }

    fn force_deploy(&self, host: &str, commit: &str) {
        self.git(&[
            "push",
            "-q",
            "origin",
            &format!("+{commit}:refs/heads/deploy/{host}"),
        ]);
    }

    fn results(&self, name: &str, overrides: &[(&str, &str)]) -> PathBuf {
        let dir = self.dir.path().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        for job in planned() {
            let (check, system) = (
                job["check"].as_str().unwrap(),
                job["system"].as_str().unwrap(),
            );
            let path = dir.join(format!("{system}-{check}.json"));
            let outcome = overrides
                .iter()
                .find(|(overridden, _)| *overridden == check)
                .map_or("passed", |(_, outcome)| *outcome);
            if outcome == "missing" {
                continue;
            }
            let result =
                json!({ "check": check, "system": system, "outcome": outcome, "excerpt": [] });
            fs::write(path, result.to_string()).unwrap();
        }
        dir
    }

    fn promote_with(&self, results: &Path, sha: &str, env: &[(&str, &str)]) -> Promoted {
        let mut command = self.command(Path::new(env!("CARGO_BIN_EXE_forge-ci")), &self.work);
        command
            .envs(IDENTITY)
            .env(
                "HOST_CHECKS",
                fs::read_to_string(Path::new(FIXTURES).join("host-checks.json")).unwrap(),
            )
            .env("PLANNED", Value::Array(planned()).to_string())
            .env("SKIPPED", SKIPPED)
            .envs(env.iter().copied())
            .arg("promote")
            .arg(results)
            .arg(sha);
        let output = command.output().unwrap();
        Promoted {
            code: output.status.code().unwrap(),
            output: format!(
                "{}{}",
                String::from_utf8(output.stdout).unwrap(),
                String::from_utf8(output.stderr).unwrap()
            ),
        }
    }

    fn promote(&self, results: &Path, sha: &str) -> Promoted {
        self.promote_with(results, sha, &[])
    }

    fn keygen(&self, name: &str) -> String {
        let path = self.dir.path().join(name);
        let output = self
            .command(&self.bin.join("real-ssh-keygen"), self.dir.path())
            .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(output.status.success(), "ssh-keygen: {output:?}");
        let public = fs::read_to_string(path.with_extension("pub")).unwrap();
        fs::write(
            self.dir.path().join(format!("{name}.allowed")),
            format!("deploy namespaces=\"git\" {public}"),
        )
        .unwrap();
        fs::read_to_string(path).unwrap()
    }

    fn verified(&self, commit: &str, signer: &str) -> bool {
        let allowed = self.dir.path().join(format!("{signer}.allowed"));
        self.command(&self.bin.join("git"), &self.work)
            .args(["-c", "gpg.format=ssh", "-c"])
            .arg(format!("gpg.ssh.allowedSignersFile={}", allowed.display()))
            .args(["verify-commit", commit])
            .output()
            .unwrap()
            .status
            .success()
    }

    fn parents(&self, commit: &str) -> String {
        let line = self.git(&["rev-list", "--parents", "-n", "1", commit]);
        line.split_once(' ')
            .map(|(_, parents)| parents.to_owned())
            .unwrap_or_default()
    }

    fn subject(&self, commit: &str) -> String {
        self.git(&["log", "-1", "--format=%s", commit])
    }

    fn tree(&self, commit: &str) -> String {
        self.git(&["rev-parse", &format!("{commit}^{{tree}}")])
    }

    fn argv_log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }

    fn tmp_is_empty(&self) -> bool {
        fs::read_dir(&self.tmp).unwrap().next().is_none()
    }
}

struct History {
    forge: Forge,
    a: String,
    b: String,
    c: String,
}

fn history() -> History {
    let forge = Forge::new();
    let a = forge.commit("A");
    let b = forge.commit("B");
    let c = forge.commit("C");
    forge.git(&["push", "-q", "origin", "main"]);
    History { forge, a, b, c }
}

fn orphan(forge: &Forge, text: &str) -> String {
    forge.git(&["checkout", "-q", "--orphan", "elsewhere"]);
    let commit = forge.commit(text);
    forge.git(&["checkout", "-q", "main"]);
    commit
}

#[test]
fn an_unsigned_promotion_moves_only_the_hosts_whose_checks_all_passed() {
    let History { forge, b, .. } = history();

    let promoted = forge.promote(
        &forge.results("first", &[("example-server-boots", "failed")]),
        &b,
    );

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    assert_eq!(forge.deploy("example").as_deref(), Some(b.as_str()));
    assert_eq!(forge.deploy("example-server"), None);
    assert_eq!(forge.deploy("example-pi"), None);
    promoted.assert_line(&format!("deploy/example -> {}", short(&b)));
    promoted.assert_line("deploy/example-server kept: example-server-boots failed");
    promoted.assert_line("deploy/example-pi kept: example-pi-sd-image skipped");
    promoted.assert_line(UNSIGNED);
    assert_eq!(promoted.output.lines().count(), 4, "{}", promoted.output);
}

#[test]
fn a_check_without_a_result_did_not_finish_and_keeps_its_host() {
    let History { forge, b, c, .. } = history();
    forge.promote(
        &forge.results("first", &[("example-server-boots", "failed")]),
        &b,
    );

    let promoted = forge.promote(&forge.results("second", &[("example-disk", "missing")]), &c);

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    assert_eq!(forge.deploy("example").as_deref(), Some(b.as_str()));
    assert_eq!(forge.deploy("example-server").as_deref(), Some(c.as_str()));
    promoted.assert_line("deploy/example kept: example-disk did not finish");
}

#[test]
fn a_result_that_is_not_json_counts_as_a_check_that_did_not_finish() {
    let History { forge, c, .. } = history();
    let results = forge.results("broken", &[]);
    fs::write(results.join("x86_64-linux-example-boots.json"), "{").unwrap();

    let promoted = forge.promote(&results, &c);

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    assert_eq!(forge.deploy("example"), None);
    assert_eq!(forge.deploy("example-server").as_deref(), Some(c.as_str()));
    promoted.assert_line("deploy/example kept: example-boots did not finish");
}

#[test]
fn a_push_that_is_not_a_fast_forward_is_refused_without_stopping_the_other_hosts() {
    let History { forge, c, .. } = history();
    forge.promote(&forge.results("first", &[]), &c);
    let d = orphan(&forge, "D");
    forge.force_deploy("example", &d);

    let promoted = forge.promote(&forge.results("third", &[]), &c);

    assert_eq!(promoted.code, 1, "{}", promoted.output);
    assert_eq!(forge.deploy("example").as_deref(), Some(d.as_str()));
    promoted.assert_line("deploy/example refused: not a fast-forward");
    promoted.assert_line(&format!(
        "deploy/example-server already at or past {}",
        short(&c)
    ));
}

#[test]
fn an_ancestor_of_every_deploy_branch_is_already_there() {
    let History { forge, a, b, c } = history();
    forge.promote(&forge.results("first", &[]), &c);
    forge.force_deploy("example", &b);

    let promoted = forge.promote(&forge.results("third", &[]), &a);

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    promoted.assert_line(&format!("deploy/example already at or past {}", short(&a)));
    promoted.assert_line(&format!(
        "deploy/example-server already at or past {}",
        short(&a)
    ));
    assert_eq!(forge.deploy("example").as_deref(), Some(b.as_str()));
    assert_eq!(forge.deploy("example-server").as_deref(), Some(c.as_str()));
}

#[test]
fn a_push_declined_by_the_remote_is_refused_with_its_reason() {
    let History { forge, c, .. } = history();
    forge.promote(&forge.results("first", &[]), &c);
    let e = forge.commit("E");
    write_executable(
        &forge.origin.join("hooks/pre-receive"),
        "#!/bin/sh\necho \"remote: deploy branches are frozen\" >&2\nexit 1\n",
    );

    let promoted = forge.promote(&forge.results("third", &[]), &e);

    assert_eq!(promoted.code, 1, "{}", promoted.output);
    assert_eq!(forge.deploy("example-server").as_deref(), Some(c.as_str()));
    promoted.assert_line(
        "deploy/example-server refused: [remote rejected] (pre-receive hook declined)",
    );
}

#[test]
fn every_input_must_hold_the_json_type_of_ci_plan() {
    let History { forge, c, .. } = history();
    let results = forge.results("inputs", &[]);
    for (name, wrong) in [
        ("HOST_CHECKS", "[]"),
        ("PLANNED", "{}"),
        ("SKIPPED", "not json"),
        ("PLANNED", ""),
    ] {
        let promoted = forge.promote_with(&results, &c, &[(name, wrong)]);
        assert_eq!(promoted.code, 2, "{name}={wrong}: {}", promoted.output);
        promoted.assert_line(&format!(
            "ci-promote: {name} must hold the {name} JSON of ci-plan"
        ));
    }
    assert_eq!(forge.deploy("example"), None);
}

#[test]
fn promote_takes_a_results_dir_and_a_commit() {
    let History { forge, c, .. } = history();
    for args in [vec![], vec!["results"], vec!["results", &c, "extra"]] {
        let output = forge
            .command(Path::new(env!("CARGO_BIN_EXE_forge-ci")), &forge.work)
            .arg("promote")
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "usage: ci-promote <results-dir> <sha>\n"
        );
    }

    let promoted = forge.promote(&forge.results("first", &[]), "no-such-commit");
    assert_ne!(promoted.code, 0, "{}", promoted.output);
    assert_eq!(forge.deploy("example"), None);
}

struct Signed {
    forge: Forge,
    key: String,
    c: String,
    d: String,
}

fn signed() -> Signed {
    let History { forge, c, .. } = history();
    let d = orphan(&forge, "D");
    let key = forge.keygen("signing");
    Signed { forge, key, c, d }
}

impl Signed {
    fn promote(&self, sha: &str) -> Promoted {
        self.promote_as(sha, &self.key)
    }

    fn promote_as(&self, sha: &str, key: &str) -> Promoted {
        let results = self.forge.results("signed", &[]);
        self.forge
            .promote_with(&results, sha, &[("DEPLOY_SIGNING_KEY", key)])
    }

    fn publish(&self, text: &str) -> String {
        let commit = self.forge.commit(text);
        self.forge.git(&["push", "-q", "origin", "main"]);
        commit
    }
}

#[test]
fn signed_promotions_build_on_each_other_and_only_move_forward() {
    let signed = signed();
    let Signed { forge, c, d, .. } = &signed;

    let first = signed.promote(c);
    assert_eq!(first.code, 0, "{}", first.output);
    let p1 = forge.deploy("example").unwrap();
    assert_ne!(&p1, c);
    assert!(forge.verified(&p1, "signing"));
    assert_eq!(&forge.parents(&p1), c);
    assert_eq!(forge.tree(&p1), forge.tree(c));
    assert_eq!(
        forge.subject(&p1),
        format!("deploy example: checks passed for {}", short(c))
    );
    first.assert_line(&format!(
        "deploy/example -> {}, signed as {}",
        short(c),
        short(&p1)
    ));
    assert!(!first.output.contains("unsigned"), "{}", first.output);

    let f = signed.publish("F");
    let second = signed.promote(&f);
    assert_eq!(second.code, 0, "{}", second.output);
    let p2 = forge.deploy("example").unwrap();
    assert!(forge.verified(&p2, "signing"));
    assert_eq!(forge.parents(&p2), format!("{p1} {f}"));
    assert_eq!(forge.tree(&p2), forge.tree(&f));

    let back = signed.promote(c);
    assert_eq!(back.code, 0, "{}", back.output);
    back.assert_line(&format!("deploy/example already at or past {}", short(c)));
    assert_eq!(forge.deploy("example"), Some(p2.clone()));

    let sideways = signed.promote(d);
    assert_eq!(sideways.code, 1, "{}", sideways.output);
    sideways.assert_line("deploy/example refused: not a fast-forward");
    assert_eq!(forge.deploy("example"), Some(p2));
}

#[test]
fn a_signed_promotion_replaces_a_tip_the_key_did_not_sign_and_warns_in_github_actions() {
    let signed = signed();
    let Signed { forge, c, d, .. } = &signed;
    signed.promote(c);
    let f = signed.publish("F");
    signed.promote(&f);
    let p2 = forge.deploy("example").unwrap();
    let forged = forge.git(&[
        "commit-tree",
        "-p",
        &p2,
        "-p",
        d,
        "-m",
        &format!("deploy example: checks passed for {}", short(d)),
        &format!("{d}^{{tree}}"),
    ]);
    forge.force_deploy("example", &forged);

    let results = forge.results("signed", &[]);
    let promoted = forge.promote_with(
        &results,
        &f,
        &[
            ("DEPLOY_SIGNING_KEY", &signed.key),
            ("GITHUB_ACTIONS", "true"),
        ],
    );

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    let warning = format!(
        "deploy/example warning: its tip {} is not a promotion signed by DEPLOY_SIGNING_KEY, so the promotion replaces it instead of building on it",
        short(&forged)
    );
    promoted.assert_line(&warning);
    promoted.assert_line(&format!("::warning::{warning}"));
    let p3 = forge.deploy("example").unwrap();
    assert!(forge.verified(&p3, "signing"));
    assert_eq!(forge.parents(&p3), f);
}

#[test]
fn a_tip_reset_to_main_is_not_already_promoted() {
    let signed = signed();
    let Signed { forge, c, .. } = &signed;
    signed.promote(c);
    let f = signed.publish("F");
    forge.force_deploy("example", &f);

    let promoted = signed.promote(&f);

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    assert!(
        !promoted
            .output
            .contains("deploy/example already at or past"),
        "{}",
        promoted.output
    );
    assert!(
        !promoted.output.contains("::warning::"),
        "{}",
        promoted.output
    );
    let p4 = forge.deploy("example").unwrap();
    assert!(forge.verified(&p4, "signing"));
    assert_eq!(forge.parents(&p4), f);
}

#[test]
fn a_key_that_cannot_sign_refuses_every_host_without_printing_it() {
    let signed = signed();
    let Signed { forge, c, .. } = &signed;
    signed.promote(c);
    let p1 = forge.deploy("example").unwrap();
    let h = signed.publish("H");

    let promoted = signed.promote_as(&h, "not a key");

    assert_eq!(promoted.code, 1, "{}", promoted.output);
    promoted.assert_line(CANNOT_SIGN);
    assert!(
        !promoted.output.contains("not a key"),
        "{}",
        promoted.output
    );
    assert_eq!(forge.deploy("example"), Some(p1));
    assert!(forge.tmp_is_empty());
}

#[test]
fn a_promotion_signed_by_another_key_does_not_verify() {
    let signed = signed();
    let Signed { forge, c, .. } = &signed;
    signed.promote(c);
    let other = forge.keygen("other");
    let g = signed.publish("G");

    let promoted = signed.promote_as(&g, &other);

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    let tip = forge.deploy("example").unwrap();
    assert!(forge.verified(&tip, "other"));
    assert!(!forge.verified(&tip, "signing"));
}

#[test]
fn the_signing_key_stays_private_and_is_removed_after_the_run() {
    let signed = signed();
    let Signed { forge, c, key, .. } = &signed;

    let promoted = signed.promote(c);

    assert_eq!(promoted.code, 0, "{}", promoted.output);
    let log = forge.argv_log();
    let secret = key.lines().find(|line| !line.starts_with("-----")).unwrap();
    assert!(!log.contains(secret), "the key is in an argv:\n{log}");
    assert!(!promoted.output.contains(secret), "{}", promoted.output);
    let modes: Vec<&str> = log
        .lines()
        .filter(|line| line.starts_with("600 ") || line.starts_with("700 "))
        .collect();
    assert!(
        modes
            .iter()
            .any(|line| line.starts_with("600 ") && line.ends_with("/key")),
        "no key file of mode 600:\n{log}"
    );
    assert!(
        modes.iter().any(|line| line.starts_with("700 ")),
        "no 700 dir:\n{log}"
    );
    let work = forge.work.to_string_lossy();
    assert!(
        modes.iter().all(|line| !line.contains(&*work)),
        "the key is written in the repository:\n{log}"
    );
    assert!(forge.tmp_is_empty(), "the key dir is left behind");
}

#[test]
fn a_repository_without_an_identity_signs_as_ci_promote() {
    let signed = signed();
    let Signed { forge, c, key, .. } = &signed;
    let results = forge.results("signed", &[]);
    let output = forge
        .command(Path::new(env!("CARGO_BIN_EXE_forge-ci")), &forge.work)
        .env(
            "HOST_CHECKS",
            fs::read_to_string(Path::new(FIXTURES).join("host-checks.json")).unwrap(),
        )
        .env("PLANNED", Value::Array(planned()).to_string())
        .env("SKIPPED", SKIPPED)
        .env("DEPLOY_SIGNING_KEY", key)
        .arg("promote")
        .arg(&results)
        .arg(c)
        .output()
        .unwrap();

    assert!(output.status.success(), "{output:?}");
    let tip = forge.deploy("example").unwrap();
    assert_eq!(
        forge.git(&["log", "-1", "--format=%cn <%ce>", &tip]),
        "ci-promote <ci-promote@localhost>"
    );
    assert!(forge.verified(&tip, "signing"));
}
