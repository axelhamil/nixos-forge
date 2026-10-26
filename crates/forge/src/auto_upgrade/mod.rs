mod apply;
mod config;
mod decision;
mod signature;
mod sources;
mod state;
mod trust;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub use config::{CONFIG_VARIABLE, Config, Operation};
pub use state::{HISTORY_FILE, Outcome, Row, STATE_FILE, State, parse_history_line};

use apply::Failure;
use decision::{Decision, GitHistory, Revisions};
use signature::Signers;
use sources::Found;
use trust::Checkout;

use crate::plugins;
use crate::system::{
    Accounts, Clock, Commands, Files, Git, Invocation, Streams, boot_entries, generation_went_back,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("git {0} failed")]
    Git(String),
    #[error("cannot {action} {}: {source}", path.display())]
    Io {
        action: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
}

pub struct Ports<'a> {
    pub commands: &'a dyn Commands,
    pub streams: &'a dyn Streams,
    pub clock: &'a dyn Clock,
    pub accounts: &'a dyn Accounts,
    pub files: &'a dyn Files,
}

pub struct Paths {
    pub current_system: PathBuf,
    pub system_profile: PathBuf,
    pub running_variant: PathBuf,
}

impl Default for Paths {
    fn default() -> Self {
        Self {
            current_system: PathBuf::from("/run/current-system"),
            system_profile: PathBuf::from("/nix/var/nix/profiles/system"),
            running_variant: PathBuf::from("/etc/specialisation"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub current_rev: Option<String>,
    pub next_rev: Option<String>,
    pub next_did_not_start: Option<String>,
}

struct Run<'a> {
    config: &'a Config,
    ports: &'a Ports<'a>,
    paths: &'a Paths,
    out: &'a mut dyn Write,
    state: State,
}

impl Run<'_> {
    fn finish(&mut self, outcome: Outcome, message: &str) -> Result<Outcome, Error> {
        self.state.outcome = outcome;
        message.clone_into(&mut self.state.message);
        let dir = &self.config.state_dir;
        self.state.record(dir).map_err(|source| Error::Io {
            action: "record the state in",
            path: dir.clone(),
            source,
        })?;
        let _ = self.out.write_all(self.state.summary().as_bytes());
        Ok(outcome)
    }
}

pub fn run(
    config: &Config,
    ports: &Ports,
    paths: &Paths,
    overrides: &Overrides,
    out: &mut dyn Write,
) -> Result<Option<Outcome>, Error> {
    let lock = state::lock_path(&config.state_dir);
    let Some(_held) = state::lock(&config.state_dir).map_err(|source| Error::Io {
        action: "lock",
        path: lock.clone(),
        source,
    })?
    else {
        let _ = writeln!(
            out,
            "another auto-upgrade run holds {}: this run stops without changing anything",
            lock.display()
        );
        return Ok(None);
    };

    let mut run = Run {
        config,
        ports,
        paths,
        out,
        state: State {
            time: state::iso_time(ports.clock.now(), &ports.clock.zone()),
            outcome: Outcome::Failed,
            from: String::new(),
            to: String::new(),
            source: String::new(),
            message: String::new(),
            changes: String::new(),
        },
    };
    attempt(&mut run, overrides).map(Some)
}

fn attempt(run: &mut Run, overrides: &Overrides) -> Result<Outcome, Error> {
    let config = run.config;
    let ports = run.ports;
    let paths = run.paths;

    let mut candidates = Vec::new();
    if let Some(checkout) = &config.checkout {
        match trust::inspect(checkout, &ports.accounts.directory(), ports.commands) {
            Ok(Checkout::Origin(url)) => candidates.push(url),
            Ok(Checkout::Absent | Checkout::NoOrigin) => {}
            Ok(Checkout::Untrusted) => {
                return run.finish(Outcome::Refused, &decision::untrusted(checkout));
            }
            Ok(Checkout::Cleartext) => {
                return run.finish(Outcome::Refused, &decision::cleartext(checkout));
            }
            Err(source) => {
                return Err(Error::Io {
                    action: "open the checkout",
                    path: checkout.clone(),
                    source,
                });
            }
        }
    }
    candidates.extend(config.sources.iter().filter(|url| !url.is_empty()).cloned());

    let repo = config.state_dir.join("repo");
    init_repo(ports, &repo)?;
    clear_stale_locks(ports.files, &repo);

    let branch = config.branch();
    let git = Git::new(ports.commands, &repo.display().to_string());
    let signers = config
        .allowed_signers
        .as_deref()
        .map(|allowed| Signers::new(&git, &config.host, allowed));
    let (source, to) = match sources::find(&git, &candidates, &branch, signers.as_ref())? {
        Found::Source { source, to } => (source, to),
        missing => {
            let (outcome, message) = decision::not_found(missing, &branch);
            return run.finish(outcome, &message);
        }
    };
    run.state.source.clone_from(&source);
    run.state.to.clone_from(&to);

    let running = overrides
        .current_rev
        .clone()
        .unwrap_or_else(|| revision_of(ports.commands, &paths.current_system));
    let next = overrides
        .next_rev
        .clone()
        .unwrap_or_else(|| revision_of(ports.commands, &paths.system_profile));
    run.state.from.clone_from(&running);
    let (next, failed) = decision::settle(&running, &next, || {
        next_did_not_start(ports, paths, overrides)
    });

    let revisions = Revisions {
        running: &running,
        next,
        failed,
        to: &to,
    };
    let history = GitHistory {
        git: &git,
        host: &config.host,
    };
    let decision = decision::decide(&revisions, &history);
    if decision != Decision::Uncommitted {
        keep(&git, "current", &running);
        keep(&git, "next", next);
    }
    let short = || git.loud(&["rev-parse", "--short", &to]).line().to_owned();
    if let Some((outcome, message)) = decision::verdict(decision, &branch, short) {
        return run.finish(outcome, &message);
    }

    upgrade(run, &git, &to)
}

fn init_repo(ports: &Ports, repo: &Path) -> Result<(), Error> {
    if ports.files.is_dir(repo) {
        return Ok(());
    }
    let fresh = repo.with_extension("new");
    match fs::remove_dir_all(&fresh) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(Error::Io {
                action: "remove",
                path: fresh,
                source,
            });
        }
    }

    let fresh_arg = fresh.display().to_string();
    let init = ports
        .commands
        .run(&Invocation::new("git", &["init", "--quiet", "--bare", &fresh_arg]).loud());
    if !init.success {
        return Err(Error::Git(format!("init {}", repo.display())));
    }
    fs::rename(&fresh, repo).map_err(|source| Error::Io {
        action: "move the new repository to",
        path: repo.to_path_buf(),
        source,
    })
}

fn clear_stale_locks(files: &dyn Files, repo: &Path) {
    let candidates = repo.join("refs/candidates");
    for name in files.list(&candidates) {
        let path = candidates.join(name);
        if path
            .extension()
            .is_some_and(|extension| extension == "lock")
        {
            let _ = fs::remove_file(path);
        }
    }
}

fn upgrade(run: &mut Run, git: &Git, to: &str) -> Result<Outcome, Error> {
    let config = run.config;
    let ports = run.ports;
    let paths = run.paths;

    let variant = apply::kept_variant(
        ports.commands,
        ports.files,
        config,
        git.repo(),
        to,
        &paths.running_variant,
    );
    let mut new_system = paths.system_profile.clone();
    if let Some(variant) = &variant {
        new_system = new_system.join("specialisation").join(variant);
    }
    let old_system = ports.files.canonicalize(&paths.current_system);

    let log = config.state_dir.join("rebuild.log");
    let invocation = apply::rebuild(config, git.repo(), to, variant.as_deref());
    let streamed = ports
        .streams
        .stream(&invocation, &log)
        .map_err(|source| Error::Io {
            action: "run the rebuild, logging to",
            path: log.clone(),
            source,
        })?;
    if let Err(error) = &streamed.logged {
        eprintln!(
            "forge auto-upgrade: cannot write {}, the rebuild went on without it: {error}",
            log.display()
        );
    }
    if !streamed.succeeded {
        let output = ports.files.read(&log).unwrap_or_default();
        let cache_miss = streamed.logged.is_ok()
            && apply::classify(&String::from_utf8_lossy(&output)) == Failure::CacheLacksVersion;
        if cache_miss {
            return run.finish(Outcome::Waiting, "the cache does not have this version yet");
        }
        return run.finish(
            Outcome::Failed,
            &format!(
                "nixos-rebuild {} failed: journalctl -u auto-upgrade shows why",
                config.operation.as_str()
            ),
        );
    }

    keep(git, "next", to);
    if config.operation != Operation::Boot {
        keep(git, "current", to);
    }

    run.state.changes = apply::closure_changes(ports.commands, &old_system, &new_system);
    let message = match config.operation {
        Operation::Boot => "applies at next boot",
        Operation::Switch => "running now",
    };
    run.finish(Outcome::Upgraded, message)
}

fn keep(git: &Git, name: &str, commit: &str) {
    let reference = format!("refs/auto-upgrade/{name}");
    let _ = git.quiet(&["update-ref", &reference, commit]);
}

fn revision_of(commands: &dyn Commands, system: &Path) -> String {
    let program = system.join("sw/bin/nixos-version");
    if !plugins::is_executable(&program) {
        return String::new();
    }
    let program = program.display().to_string();
    commands
        .run(&Invocation::new(&program, &["--configuration-revision"]))
        .line()
        .to_owned()
}

fn next_did_not_start(ports: &Ports, paths: &Paths, overrides: &Overrides) -> bool {
    if let Some(value) = &overrides.next_did_not_start {
        return value == "true";
    }
    let Some(generation) = ports
        .files
        .read_link(&paths.system_profile)
        .and_then(|link| profile_generation(&link.to_string_lossy()))
    else {
        return false;
    };
    let profiles = paths.system_profile.parent().unwrap_or(Path::new("/"));
    boot_entries(ports.commands)
        .iter()
        .filter(|entry| entry.generation == generation)
        .any(|entry| generation_went_back(ports.files, entry, profiles, &paths.current_system))
}

fn profile_generation(link: &str) -> Option<u64> {
    let digits = link.strip_prefix("system-")?.strip_suffix("-link")?;
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use jiff::Timestamp;
    use jiff::tz::TimeZone;
    use tempfile::TempDir;

    use super::*;
    use crate::system::{Answer, Directory, Owner, Streamed};
    use decision::WAITING;

    const LIST: &str =
        "git ls-remote --quiet --exit-code --end-of-options https://a refs/heads/deploy/machine";

    struct Machine {
        dir: TempDir,
        answers: HashMap<String, Answer>,
        rebuild: Option<&'static str>,
        rebuilt: RefCell<Vec<String>>,
        init_fails: bool,
    }

    impl Machine {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            fs::create_dir(dir.path().join("repo")).unwrap();
            Self {
                dir,
                answers: HashMap::new(),
                rebuild: None,
                rebuilt: RefCell::new(Vec::new()),
                init_fails: false,
            }
        }

        fn git(&self, args: &str) -> String {
            format!("git -C {}/repo {args}", self.dir.path().display())
        }

        fn answer(mut self, command_line: String, answer: Answer) -> Self {
            self.answers.insert(command_line, answer);
            self
        }

        fn serving(self, commits: &[(&str, &str)]) -> Self {
            let fetch = self.git("fetch --quiet --no-tags --end-of-options https://a +refs/heads/deploy/machine:refs/candidates/0");
            let parse = self.git("rev-parse refs/candidates/0");
            let mut machine = self
                .answer(LIST.to_owned(), Answer::ok(""))
                .answer(fetch, Answer::ok(""))
                .answer(
                    parse,
                    Answer::ok(&format!("{}\n", commits[commits.len() - 1].0)),
                );
            for (index, (commit, subject)) in commits.iter().enumerate() {
                let log = machine.git(&format!("log -1 --format=%s {commit}"));
                machine = machine.answer(log, Answer::ok(&format!("{subject}\n")));
                for (older, _) in &commits[..=index] {
                    let ancestry =
                        machine.git(&format!("merge-base --is-ancestor {older} {commit}"));
                    machine = machine.answer(ancestry, Answer::ok(""));
                }
            }
            machine
        }

        fn attempt(
            &self,
            current: &str,
            next: &str,
            started: bool,
            operation: Operation,
        ) -> (Result<Option<Outcome>, Error>, String) {
            let config = Config {
                host: "machine".to_owned(),
                state_dir: self.dir.path().to_path_buf(),
                checkout: None,
                sources: vec!["https://a".to_owned()],
                operation,
                rebuild_command: "rebuild".to_owned(),
                allow_local_build: false,
                allowed_signers: None,
            };
            let ports = Ports {
                commands: self,
                streams: self,
                clock: self,
                accounts: self,
                files: self,
            };
            let paths = Paths {
                current_system: self.dir.path().join("current-system"),
                system_profile: self.dir.path().join("system"),
                running_variant: self.dir.path().join("specialisation"),
            };
            let overrides = Overrides {
                current_rev: Some(current.to_owned()),
                next_rev: Some(next.to_owned()),
                next_did_not_start: Some((!started).to_string()),
            };
            let mut out = Vec::new();
            let result = run(&config, &ports, &paths, &overrides, &mut out);
            (result, String::from_utf8(out).unwrap())
        }

        fn run(
            &self,
            current: &str,
            next: &str,
            started: bool,
            operation: Operation,
        ) -> (Outcome, State, String) {
            let (result, out) = self.attempt(current, next, started, operation);
            let outcome = result.unwrap().unwrap();
            let state = State::load(&fs::read(self.dir.path().join(STATE_FILE)).unwrap()).unwrap();
            assert_eq!(state.outcome, outcome);
            (outcome, state, out)
        }
    }

    impl Commands for Machine {
        fn run(&self, invocation: &Invocation) -> Answer {
            if let Some(target) = invocation
                .command_line()
                .strip_prefix("git init --quiet --bare ")
            {
                fs::create_dir(target).unwrap();
                return Answer::exited(i32::from(self.init_fails));
            }
            self.answers
                .get(&invocation.command_line())
                .cloned()
                .unwrap_or_else(|| Answer::exited(128))
        }
    }

    impl Streams for Machine {
        fn stream(&self, invocation: &Invocation, log: &Path) -> io::Result<Streamed> {
            self.rebuilt.borrow_mut().push(invocation.command_line());
            fs::write(log, self.rebuild.unwrap_or_default())?;
            Ok(Streamed {
                succeeded: self.rebuild.is_none(),
                logged: Ok(()),
            })
        }
    }

    impl Clock for Machine {
        fn now(&self) -> Timestamp {
            "2026-10-23T02:00:00Z".parse().unwrap()
        }

        fn zone(&self) -> TimeZone {
            TimeZone::UTC
        }
    }

    impl Files for Machine {
        fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
            crate::system::Machine.read(path)
        }

        fn read_link(&self, path: &Path) -> Option<PathBuf> {
            crate::system::Machine.read_link(path)
        }

        fn is_dir(&self, path: &Path) -> bool {
            crate::system::Machine.is_dir(path)
        }

        fn list(&self, dir: &Path) -> Vec<String> {
            crate::system::Machine.list(dir)
        }

        fn canonicalize(&self, path: &Path) -> PathBuf {
            crate::system::Machine.canonicalize(path)
        }

        fn owner(&self, path: &Path) -> Option<Owner> {
            crate::system::Machine.owner(path)
        }
    }

    impl Accounts for Machine {
        fn directory(&self) -> Directory {
            Directory::default()
        }
    }

    #[test]
    fn a_source_without_the_branch_waits_for_ci() {
        let machine = Machine::new().answer(LIST.to_owned(), Answer::exited(2));

        let (outcome, state, out) = machine.run("a", "a", true, Operation::Boot);

        assert_eq!(outcome, Outcome::Waiting);
        assert_eq!(state.message, WAITING);
        assert_eq!(out, format!("waiting: {WAITING}\n"));
        assert_eq!(state.time, "2026-10-23T02:00:00+00:00");
    }

    #[test]
    fn no_answering_source_is_unreachable() {
        let (outcome, state, _) = Machine::new().run("a", "a", true, Operation::Boot);

        assert_eq!(outcome, Outcome::Unreachable);
        assert_eq!(
            state.message,
            "no source reachable: this machine stays on its version"
        );
    }

    #[test]
    fn a_newer_commit_is_built_for_the_next_boot() {
        let machine = Machine::new().serving(&[("a", "A"), ("b", "B")]);
        let diff = format!(
            "nvd diff {0}/current-system {0}/system",
            machine.dir.path().display()
        );
        let machine = machine.answer(
            diff,
            Answer::ok("<<< old\nClosure size: 10 -> 11 (2 paths added)\n"),
        );

        let (outcome, state, out) = machine.run("a", "a", true, Operation::Boot);

        assert_eq!(outcome, Outcome::Upgraded);
        assert_eq!(
            (
                state.from.as_str(),
                state.to.as_str(),
                state.source.as_str()
            ),
            ("a", "b", "https://a")
        );
        assert_eq!(state.message, "applies at next boot");
        assert_eq!(state.changes, "Closure size: 10 -> 11 (2 paths added)");
        assert_eq!(out, "upgraded: applies at next boot\n");
        assert_eq!(
            machine.rebuilt.borrow().as_slice(),
            [format!(
                "rebuild boot --flake git+file://{}/repo?rev=b#machine --option max-jobs 0 --option always-allow-substitutes true",
                machine.dir.path().display()
            )]
        );
    }

    #[test]
    fn a_cache_miss_waits_and_another_failure_fails() {
        let mut machine = Machine::new().serving(&[("a", "A"), ("b", "B")]);
        machine.rebuild = Some("error: local builds are disabled\n");

        let (outcome, state, _) = machine.run("a", "a", true, Operation::Boot);
        assert_eq!(outcome, Outcome::Waiting);
        assert_eq!(state.message, "the cache does not have this version yet");

        machine.rebuild = Some("error: attribute 'machine' missing\n");
        let (outcome, state, _) = machine.run("a", "a", true, Operation::Switch);
        assert_eq!(outcome, Outcome::Failed);
        assert_eq!(
            state.message,
            "nixos-rebuild switch failed: journalctl -u auto-upgrade shows why"
        );
    }

    #[test]
    fn the_running_or_installed_commit_is_up_to_date() {
        let machine = Machine::new().serving(&[("a", "A"), ("b", "B")]);

        let (outcome, state, _) = machine.run("b", "b", true, Operation::Boot);
        assert_eq!(outcome, Outcome::UpToDate);
        assert_eq!(state.message, "deploy/machine is the running commit");

        let (outcome, state, _) = machine.run("a", "b", true, Operation::Boot);
        assert_eq!(outcome, Outcome::UpToDate);
        assert_eq!(
            state.message,
            "deploy/machine is installed and applies at next boot"
        );
        assert!(machine.rebuilt.borrow().is_empty());
    }

    #[test]
    fn the_commit_that_did_not_start_is_rolled_back() {
        let machine = Machine::new().serving(&[("a", "A"), ("b", "B")]);
        let short = machine.git("rev-parse --short b");
        let machine = machine.answer(short, Answer::ok("b123456\n"));

        let (outcome, state, _) = machine.run("a", "b", false, Operation::Boot);

        assert_eq!(outcome, Outcome::RolledBack);
        assert_eq!(
            state.message,
            "b123456 did not start and the machine went back to the previous generation: fix it and push, CI will validate the fix"
        );
        assert_eq!(state.from, "a");
    }

    #[test]
    fn uncommitted_or_unrelated_revisions_are_refused() {
        let machine = Machine::new().serving(&[("c", "C")]);

        let (outcome, state, _) = machine.run("a-dirty", "a-dirty", true, Operation::Boot);
        assert_eq!(outcome, Outcome::Refused);
        assert_eq!(
            state.message,
            "this machine runs uncommitted changes: commit and push them, CI will validate them"
        );

        let machine = machine.serving(&[("a", "A")]).serving(&[("c", "C")]);
        let (outcome, state, _) = machine.run("a", "a", true, Operation::Boot);
        assert_eq!(outcome, Outcome::Refused);
        assert_eq!(
            state.message,
            "deploy/machine does not contain the main commit this machine runs: push it, or the pointer was moved back"
        );
        assert!(machine.rebuilt.borrow().is_empty());
    }

    #[test]
    fn an_interrupted_init_leaves_no_repository_behind() {
        let mut machine = Machine::new();
        fs::remove_dir(machine.dir.path().join("repo")).unwrap();
        machine.init_fails = true;

        let (result, _) = machine.attempt("a", "a", true, Operation::Boot);

        assert!(result.is_err());
        assert!(!machine.dir.path().join("repo").exists());
    }

    #[test]
    fn an_init_left_half_done_is_started_again() {
        let machine = Machine::new().answer(LIST.to_owned(), Answer::exited(2));
        fs::remove_dir(machine.dir.path().join("repo")).unwrap();
        fs::create_dir(machine.dir.path().join("repo.new")).unwrap();

        let (outcome, _, _) = machine.run("a", "a", true, Operation::Boot);

        assert_eq!(outcome, Outcome::Waiting);
        assert!(machine.dir.path().join("repo").is_dir());
        assert!(!machine.dir.path().join("repo.new").exists());
    }

    #[test]
    fn a_run_while_another_holds_the_lock_changes_nothing() {
        let machine = Machine::new().serving(&[("a", "A"), ("b", "B")]);
        let lock = fs::File::create(machine.dir.path().join("lock")).unwrap();
        rustix::fs::flock(&lock, rustix::fs::FlockOperation::NonBlockingLockExclusive).unwrap();

        let (result, out) = machine.attempt("a", "a", true, Operation::Boot);

        assert!(matches!(result, Ok(None)));
        assert_eq!(
            out,
            format!(
                "another auto-upgrade run holds {}/lock: this run stops without changing anything\n",
                machine.dir.path().display()
            )
        );
        assert!(!machine.dir.path().join("state.json").exists());
        assert!(machine.rebuilt.borrow().is_empty());
    }

    #[test]
    fn a_stale_candidate_lock_is_removed_before_fetching() {
        let machine = Machine::new().answer(LIST.to_owned(), Answer::exited(2));
        let candidates = machine.dir.path().join("repo/refs/candidates");
        fs::create_dir_all(&candidates).unwrap();
        fs::write(candidates.join("0.lock"), "").unwrap();

        machine.run("a", "a", true, Operation::Boot);

        assert!(!candidates.join("0.lock").exists());
    }

    #[test]
    fn only_a_numbered_system_link_names_a_generation() {
        assert_eq!(profile_generation("system-12-link"), Some(12));
        assert_eq!(profile_generation("system--link"), None);
        assert_eq!(
            profile_generation("/nix/var/nix/profiles/system-12-link"),
            None
        );
        assert_eq!(profile_generation("system-1x-link"), None);
    }
}
