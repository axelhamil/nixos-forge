use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::time::Duration;

use forge::system::{Answer, Commands, Invocation};

pub const USAGE: &str =
    "usage: stow-dotfiles prune|check|apply <dotfiles> <target> <state> [--dotfiles] [package...]";

const MIRROR: &str = ".";
const SIMULATE_TIMEOUT: Duration = Duration::from_secs(600);

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("cannot link {path} to the dotfiles: {source}")]
    Mirror { path: PathBuf, source: io::Error },
    #[error("cannot write {path}: {source}")]
    State { path: PathBuf, source: io::Error },
}

pub struct Environment<'a, O: Write, E: Write> {
    pub user: &'a str,
    pub out: O,
    pub err: E,
}

struct Stow<'a, C: Commands> {
    commands: &'a C,
    directory: &'a Path,
    target: &'a OsStr,
    state: &'a Path,
    dotfiles: bool,
}

pub fn run<C: Commands, O: Write, E: Write>(
    commands: &C,
    args: &[OsString],
    mut env: Environment<'_, O, E>,
) -> u8 {
    let [mode, directory, target, state, rest @ ..] = args else {
        return usage(&mut env.err);
    };
    let (dotfiles, named) = match rest {
        [flag, named @ ..] if flag == "--dotfiles" => (true, named),
        _ => (false, rest),
    };
    let directory = Path::new(directory);

    if !directory.is_dir() {
        if mode == "apply" {
            let _ = writeln!(
                env.out,
                "stow: no dotfiles in {} yet; clone them there, then run: systemctl restart home-manager-{}",
                directory.display(),
                env.user
            );
        }
        return 0;
    }

    let stow = Stow {
        commands,
        directory,
        target,
        state: Path::new(state),
        dotfiles,
    };
    let requested = if !named.is_empty() {
        named.to_vec()
    } else if mirrors_home(directory) {
        vec![OsString::from(MIRROR)]
    } else {
        top_level_directories(directory)
    };
    let packages: Vec<OsString> = requested
        .into_iter()
        .filter(|package| {
            let present = stow.has(package);
            if !present {
                let _ = writeln!(
                    env.err,
                    "stow: no package {} in {}, skipped",
                    package.to_string_lossy(),
                    directory.display()
                );
            }
            present
        })
        .collect();

    match mode.to_str() {
        Some("prune") => stow.prune(&packages, &mut env.err),
        Some("check") => stow.check(&packages, &mut env.err),
        Some("apply") => stow.apply(&packages, env.user, &mut env.err),
        _ => usage(&mut env.err),
    }
}

fn usage(err: &mut impl Write) -> u8 {
    let _ = writeln!(err, "{USAGE}");
    2
}

fn mirrors_home(directory: &Path) -> bool {
    let Ok(entries) = fs::read_dir(directory) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.as_bytes();
        name.starts_with(b".") && name != b".git" && !name.starts_with(b".stow")
    })
}

fn top_level_directories(directory: &Path) -> Vec<OsString> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut names: Vec<OsString> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name())
        .filter(|name| !name.as_bytes().starts_with(b"."))
        .collect();
    names.sort();
    names
}

fn state_directory(state: &Path) -> &Path {
    state.parent().unwrap_or(state)
}

fn is_mirror(package: &OsStr) -> bool {
    package == MIRROR
}

fn report_failure(answer: &Answer, err: &mut impl Write) {
    if !answer.success && !answer.stderr.is_empty() {
        let _ = write!(err, "{}", answer.stderr);
    }
}

impl<C: Commands> Stow<'_, C> {
    fn has(&self, package: &OsStr) -> bool {
        if is_mirror(package) {
            return true;
        }
        let mut path = self.directory.as_os_str().to_owned();
        path.push("/");
        path.push(package);
        Path::new(&path).is_dir()
    }

    fn invocation(&self, package: &OsStr, action: &str) -> Result<Invocation, Error> {
        let (dir, name) = if is_mirror(package) {
            (self.link_mirror()?, OsStr::new("home"))
        } else {
            (self.directory.to_path_buf(), package)
        };
        let mut args = vec![self.target];
        if self.dotfiles {
            args.push(OsStr::new("--dotfiles"));
        }
        args.extend([
            OsStr::new("--dir"),
            dir.as_os_str(),
            OsStr::new(action),
            name,
        ]);
        Ok(Invocation::new("stow", &["--no-folding", "--target"]).os_args(args))
    }

    fn link_mirror(&self) -> Result<PathBuf, Error> {
        let mirror = state_directory(self.state).join("mirror");
        let link = mirror.join("home");
        let failed = |source| Error::Mirror {
            path: link.clone(),
            source,
        };
        fs::create_dir_all(&mirror).map_err(failed)?;
        if fs::symlink_metadata(&link).is_ok_and(|metadata| !metadata.is_dir()) {
            fs::remove_file(&link).map_err(failed)?;
        }
        symlink(self.directory, &link).map_err(failed)?;
        Ok(mirror)
    }

    fn prune(&self, packages: &[OsString], err: &mut impl Write) -> u8 {
        let wanted: HashSet<&OsStr> = packages.iter().map(OsString::as_os_str).collect();
        let previous = fs::read(self.state).unwrap_or_default();

        for line in previous.split(|&byte| byte == b'\n') {
            let package = OsStr::from_bytes(line);
            if package.is_empty() || wanted.contains(package) || !self.has(package) {
                continue;
            }
            let answer = match self.invocation(package, "--delete") {
                Ok(invocation) => self.commands.run(&invocation.attached()),
                Err(error) => {
                    let _ = writeln!(err, "stow-dotfiles: {error}");
                    return 1;
                }
            };
            if !answer.success {
                report_failure(&answer, err);
                return answer.exit_code();
            }
        }
        0
    }

    fn check(&self, packages: &[OsString], err: &mut impl Write) -> u8 {
        for package in packages {
            let report = match self.invocation(package, "--simulate") {
                Ok(invocation) => {
                    let answer = self.commands.run(&invocation.timeout(SIMULATE_TIMEOUT));
                    if answer.success {
                        continue;
                    }
                    answer.stdout + &answer.stderr
                }
                Err(error) => format!("stow-dotfiles: {error}"),
            };
            let report = report.trim_end_matches('\n');

            if !report.contains("existing target") {
                let _ = writeln!(err, "{report}");
                return 1;
            }
            let _ = writeln!(
                err,
                "stow: {} has files that home-manager also writes:",
                package.to_string_lossy()
            );
            for line in report.lines() {
                if let Some(start) = line.find("existing target") {
                    let _ = writeln!(err, "{}", &line[start..]);
                }
            }
            let _ = writeln!(
                err,
                "stow: turn off the module that writes them in your home config (for instance programs.git.enable = false), or remove them from your dotfiles"
            );
            return 1;
        }
        0
    }

    fn apply(&self, packages: &[OsString], user: &str, err: &mut impl Write) -> u8 {
        for package in packages {
            let linked = match self.invocation(package, "--restow") {
                Ok(invocation) => {
                    let answer = self.commands.run(&invocation.attached());
                    report_failure(&answer, err);
                    answer.success
                }
                Err(error) => {
                    let _ = writeln!(err, "stow-dotfiles: {error}");
                    false
                }
            };
            if !linked {
                let _ = writeln!(
                    err,
                    "stow: {} not linked: move the files listed above out of the way, then run: systemctl restart home-manager-{user}",
                    package.to_string_lossy()
                );
            }
        }

        if let Err(error) = self.record(packages) {
            let _ = writeln!(err, "stow-dotfiles: {error}");
            return 1;
        }
        0
    }

    fn record(&self, packages: &[OsString]) -> Result<(), Error> {
        let failed = |source| Error::State {
            path: self.state.to_path_buf(),
            source,
        };
        let directory = state_directory(self.state);
        if !directory.as_os_str().is_empty() {
            fs::create_dir_all(directory).map_err(failed)?;
        }
        let lines: Vec<u8> = packages
            .iter()
            .flat_map(|package| package.as_bytes().iter().chain(b"\n"))
            .copied()
            .collect();
        fs::write(self.state, lines).map_err(failed)
    }
}
