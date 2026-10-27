use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::answers::{Answers, Asker, EXAMPLE_PERSON, PERSON_TEMPLATE, TEMPLATE};
use super::key::{self, DeployKey};
use super::options::Flags;
use super::render::{point_readme, render_host, render_person, update_kept_example};
use super::{Environment, Ports, Stop};
use crate::system::remote::{fork_flake, github_repository, is_upstream};
use crate::system::{Git, Invocation};

const FORMAT_TIMEOUT: Duration = Duration::from_secs(600);
const KEPT_EXAMPLES: [&str; 2] = ["example-server", "example-pi"];
const CHECKOUT: &str = ".";

pub fn init(flags: &Flags, env: &Environment, ports: &mut Ports) -> u8 {
    match steps(flags, env, ports) {
        Ok(()) => 0,
        Err(stop) => stop.exit_code(),
    }
}

fn refuse(ports: &mut Ports, stop: &Stop) {
    if let Stop::Refused(message) = stop {
        ports.warn(&format!("forge init: {message}"));
    }
}

fn steps(flags: &Flags, env: &Environment, ports: &mut Ports) -> Result<(), Stop> {
    let prepared = prepare(flags, env, ports);
    let (root, answers, templates) = prepared.inspect_err(|stop| refuse(ports, stop))?;

    let key = match key::create(ports, env) {
        Ok(key) => key,
        Err(failure) => {
            refuse(ports, &failure.stop);
            if let Some(file) = &failure.file {
                keep_message(ports, file);
            }
            return Err(failure.stop);
        }
    };

    let fork = Fork {
        root: &root,
        answers: &answers,
        key: &key,
        env,
    };
    match fork.write(ports, &templates) {
        Ok(()) => Ok(()),
        Err(stop) => {
            refuse(ports, &stop);
            keep_message(ports, &key.file);
            Err(stop)
        }
    }
}

fn keep_message(ports: &mut Ports, file: &Path) {
    ports.warn(&format!(
        "forge init: the deploy signing key is kept in {}, its public key may already be in the signers of your hosts",
        file.display()
    ));
}

struct Templates {
    host: String,
    person: String,
}

fn prepare(
    flags: &Flags,
    env: &Environment,
    ports: &mut Ports,
) -> Result<(PathBuf, Answers, Templates), Stop> {
    let root = checkout(ports)?;
    let read = |ports: &Ports, relative: &str, what: &str| {
        ports
            .files
            .read(&root.join(relative))
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|_| {
                Stop::Refused(format!(
                    "{relative} is missing: init starts from the {what}, which a previous run already replaced"
                ))
            })
    };
    let templates = Templates {
        host: read(ports, TEMPLATE, "example host")?,
        person: read(ports, PERSON_TEMPLATE, "example person")?,
    };

    let answers = Asker::new(ports, &root, env, &templates.host, flags.yes).collect(flags)?;
    ports.checkpoint()?;
    Ok((root, answers, templates))
}

fn checkout(ports: &mut Ports) -> Result<PathBuf, Stop> {
    let outside = || Stop::Refused("run it inside a git checkout of your fork".to_owned());
    let answer = ports.run(&Invocation::new(
        "git",
        &["rev-parse", "--is-inside-work-tree", "--show-cdup"],
    ))?;
    let mut lines = answer.stdout.lines();
    if !answer.success || lines.next() != Some("true") {
        return Err(outside());
    }
    let top = match lines.next().unwrap_or_default() {
        "" => Path::new(CHECKOUT),
        up => Path::new(up),
    };
    (ports.enter)(top).map_err(|_| outside())
}

#[must_use]
pub fn shell_quoted(text: &str) -> String {
    let safe = |c: char| c.is_ascii_alphanumeric() || "@%+=:,./-_".contains(c);
    if !text.is_empty() && text.chars().all(safe) {
        return text.to_owned();
    }
    if text.chars().any(char::is_control) || text.is_empty() {
        return format!("'{}'", text.replace('\'', r"'\''"));
    }
    text.chars().fold(String::new(), |mut quoted, c| {
        if !safe(c) {
            quoted.push('\\');
        }
        quoted.push(c);
        quoted
    })
}

fn git(ports: &Ports, args: &[&str]) -> Result<(), Stop> {
    let answer = Git::new(ports.commands, CHECKOUT).loud(args);
    ports.checkpoint()?;
    if answer.success {
        return Ok(());
    }
    Err(Stop::Refused(format!("git {} failed", args.join(" "))))
}

fn origin(ports: &Ports) -> Result<Option<String>, Stop> {
    let answer = Git::new(ports.commands, CHECKOUT).quiet(&["remote", "get-url", "origin"]);
    ports.checkpoint()?;
    Ok(answer.success.then(|| answer.line().to_owned()))
}

struct Fork<'f> {
    root: &'f Path,
    answers: &'f Answers,
    key: &'f DeployKey,
    env: &'f Environment,
}

impl Fork<'_> {
    fn target(&self) -> String {
        format!("modules/hosts/{}/default.nix", self.answers.host)
    }

    fn person(&self) -> String {
        format!("modules/users/{}/default.nix", self.answers.login)
    }

    fn write(&self, ports: &mut Ports, templates: &Templates) -> Result<(), Stop> {
        let (target, person) = (self.target(), self.person());
        let host = render_host(&templates.host, self.answers, &self.key.signer);
        let you = render_person(&templates.person, self.answers);
        self.install(ports, &target, &host)?;
        self.install(ports, &person, &you)?;
        self.format(ports, &[&target, &person])?;
        git(ports, &["add", "--", &target, &person])?;

        let removed = self.remove_examples(ports)?;
        let origin = origin(ports)?;
        let kept_flake = fork_flake(origin.as_deref(), &self.env.upstream);
        self.update_kept_examples(ports, &removed, &kept_flake)?;
        self.point_readme(ports)?;

        let repository = origin
            .as_deref()
            .and_then(github_repository)
            .filter(|repository| !is_upstream(repository, &self.env.upstream));
        let steps = self.next_steps(repository.as_deref());
        ports
            .out
            .write_all(steps.as_bytes())
            .and_then(|()| ports.out.flush())
            .map_err(|error| Stop::Refused(format!("cannot print the next steps: {error}")))
    }

    fn install(&self, ports: &Ports, relative: &str, text: &str) -> Result<(), Stop> {
        let path = self.root.join(relative);
        let cannot = |error| Stop::Refused(format!("cannot write {relative}: {error}"));
        if let Some(parent) = path.parent() {
            ports.storage.create_dirs(parent).map_err(cannot)?;
        }
        ports
            .storage
            .install_file(&path, text.as_bytes(), 0o644)
            .map_err(cannot)
    }

    fn rewrite(&self, ports: &Ports, relative: &str, text: &str) -> Result<(), Stop> {
        ports
            .storage
            .write(&self.root.join(relative), text.as_bytes())
            .map_err(|error| Stop::Refused(format!("cannot write {relative}: {error}")))
    }

    fn read(&self, ports: &Ports, relative: &str) -> Option<String> {
        let bytes = ports.files.read(&self.root.join(relative)).ok()?;
        Some(String::from_utf8_lossy(&bytes).into_owned())
    }

    fn format(&self, ports: &mut Ports, files: &[&str]) -> Result<(), Stop> {
        if !self.env.nix_on_path {
            return Ok(());
        }
        let args: Vec<&str> = ["fmt", "--"]
            .into_iter()
            .chain(files.iter().copied())
            .collect();
        let invocation = Invocation::new("nix", &args)
            .timeout(FORMAT_TIMEOUT)
            .discard_output();
        if !ports.run(&invocation)?.success {
            ports.warn("forge init: nix fmt failed, run it yourself");
        }
        Ok(())
    }

    fn remove_examples(&self, ports: &Ports) -> Result<Vec<&'static str>, Stop> {
        let answers = self.answers;
        let mut removed = vec!["example"];
        if !answers.keep_server {
            removed.push("example-server");
        }
        if !answers.keep_pi {
            removed.push("example-pi");
        }
        for example in &removed {
            let dir = format!("modules/hosts/{example}");
            if *example == answers.host || !ports.files.is_dir(&self.root.join(&dir)) {
                continue;
            }
            git(ports, &["rm", "-r", "-q", "--", &dir])?;
        }

        if answers.login != EXAMPLE_PERSON {
            git(
                ports,
                &[
                    "rm",
                    "-r",
                    "-q",
                    "--",
                    &format!("modules/users/{EXAMPLE_PERSON}"),
                ],
            )?;
        }
        Ok(removed)
    }

    fn update_kept_examples(
        &self,
        ports: &Ports,
        removed: &[&str],
        kept_flake: &str,
    ) -> Result<(), Stop> {
        for example in KEPT_EXAMPLES {
            if removed.contains(&example) || example == self.answers.host {
                continue;
            }
            let file = format!("modules/hosts/{example}/default.nix");
            let Some(text) = self.read(ports, &file) else {
                continue;
            };
            let updated =
                update_kept_example(&text, &self.answers.login, kept_flake, &self.key.signer);
            self.rewrite(ports, &file, &updated)?;
            git(ports, &["add", "--", &file])?;
        }
        Ok(())
    }

    fn point_readme(&self, ports: &Ports) -> Result<(), Stop> {
        let Some(text) = self.read(ports, "README.md") else {
            return Ok(());
        };
        let pointed = point_readme(&text, &self.answers.host, &self.answers.login);
        self.rewrite(ports, "README.md", &pointed)?;
        git(ports, &["add", "--", "README.md"])
    }

    fn next_steps(&self, repository: Option<&str>) -> String {
        let (target, person) = (self.target(), self.person());
        let Answers { host, login, .. } = self.answers;
        let key_file = self.key.file.display().to_string();
        let quoted = shell_quoted(&key_file);
        let repo =
            repository.map_or_else(String::new, |repository| format!(" --repo {repository}"));

        let mut text = String::new();
        let _ = write!(
            text,
            "
{target} and {person} are ready and staged; the example hosts you did not
keep are removed. Read them top to bottom: every line you may want is there, commented out.

Next steps:
  nix develop          installs the pre-push hook
  nix flake check -L   builds every host and boots it in a VM
  nix build .#nixosConfigurations.{host}.config.system.build.vm && ./result/bin/run-{host}-vm
                       boots {host} in a VM: log in as {login} with the password \"test\"
  nix build .#installer-{host}
                       the installer ISO, for a host with the disk feature (docs/tutorials/install.md)

Deploy signing key: CI signs every deploy/<host> commit with it, and auto-upgrade
applies only commits signed by its public key, written in the signers of your hosts.
The private key is in {key_file}, outside the repository and readable
by you alone. In your fork, create the environment deploy limited to the main branch
(Settings, Environments; docs/how-to/ci.md), then store the key there and delete the file:

  gh secret set DEPLOY_SIGNING_KEY --env deploy{repo} < {quoted} && shred -u {quoted}

or, on GitHub, paste the content of the file in the secret DEPLOY_SIGNING_KEY of the
environment deploy, then run: shred -u {quoted}
"
        );
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::fake::{Machine, environment, run_init, run_init_into};
    use crate::system::Answer;
    use crate::system::Storage;
    use crate::system::interrupt::Flag;
    use crate::system::prompt::Typed;

    const ARGS: [&str; 9] = [
        "--yes",
        "--host",
        "box",
        "--login",
        "bob",
        "--full-name",
        "Bob",
        "--email",
        "",
    ];

    const KEPT: &str = "forge init: the deploy signing key is kept in /run/user/1000/deploy-key.42-7/deploy, its public key may already be in the signers of your hosts\n";

    #[test]
    fn an_interrupt_once_the_key_exists_exits_130_and_says_where_the_key_is_once() {
        let flag = Flag::default();
        let mut machine = Machine::fork();
        machine.interrupt_on = Some(("git -C . add".to_owned(), flag.clone(), 2));

        let outcome = run_init(&machine, &mut Typed::new(&[]), &flag, &environment(), &ARGS);

        assert_eq!(outcome.code, 130);
        assert_eq!(outcome.err, KEPT);
        assert!(machine.exists(std::path::Path::new(
            "/run/user/1000/deploy-key.42-7/deploy"
        )));
        assert!(!machine.ran().iter().any(|line| line.contains(" rm ")));
    }

    #[test]
    fn a_failing_step_names_itself_then_the_kept_key() {
        let machine = Machine::fork().answer(
            "git -C . rm -r -q -- modules/hosts/example",
            Answer::exited(128),
        );

        let outcome = run_init(
            &machine,
            &mut Typed::new(&[]),
            &Flag::default(),
            &environment(),
            &ARGS,
        );

        assert_eq!(outcome.code, 1);
        assert_eq!(
            outcome.err,
            format!("forge init: git rm -r -q -- modules/hosts/example failed\n{KEPT}")
        );
    }

    struct Closed;

    impl std::io::Write for Closed {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn next_steps_that_cannot_be_printed_exit_1_and_say_where_the_key_is_once() {
        let machine = Machine::fork();

        let outcome = run_init_into(
            &machine,
            &mut Typed::new(&[]),
            &Flag::default(),
            &environment(),
            &ARGS,
            &mut Closed,
        );

        assert_eq!(outcome.code, 1);
        assert_eq!(
            outcome.err,
            format!("forge init: cannot print the next steps: broken pipe\n{KEPT}")
        );
    }

    #[test]
    fn nix_fmt_runs_only_when_nix_is_there_and_its_failure_only_warns() {
        let machine = Machine::fork().answer(
            "nix fmt -- modules/hosts/box/default.nix modules/users/bob/default.nix",
            Answer::exited(1),
        );
        let env = Environment {
            nix_on_path: true,
            ..environment()
        };

        let outcome = run_init(
            &machine,
            &mut Typed::new(&[]),
            &Flag::default(),
            &env,
            &ARGS,
        );

        assert_eq!(outcome.code, 0);
        assert_eq!(outcome.err, "forge init: nix fmt failed, run it yourself\n");

        let machine = Machine::fork();
        run_init(
            &machine,
            &mut Typed::new(&[]),
            &Flag::default(),
            &environment(),
            &ARGS,
        );
        assert!(!machine.ran().iter().any(|line| line.starts_with("nix ")));
    }

    #[test]
    fn the_next_steps_are_printed_on_stdout_and_never_the_private_key() {
        let machine = Machine::fork();

        let outcome = run_init(
            &machine,
            &mut Typed::new(&[]),
            &Flag::default(),
            &environment(),
            &ARGS,
        );

        insta::assert_snapshot!(outcome.out);
        assert!(!outcome.out.contains("PRIVATE"));
        assert!(machine.text("modules/hosts/box/default.nix").is_some());
    }

    #[test]
    fn a_key_path_with_shell_characters_is_quoted_in_the_commands() {
        assert_eq!(
            shell_quoted("/run/user/1000/deploy-key.1-2/deploy"),
            "/run/user/1000/deploy-key.1-2/deploy"
        );
        assert_eq!(shell_quoted("/tmp/my keys/deploy"), r"/tmp/my\ keys/deploy");
        assert_eq!(shell_quoted("/tmp/a\nb"), "'/tmp/a\nb'");
    }
}
