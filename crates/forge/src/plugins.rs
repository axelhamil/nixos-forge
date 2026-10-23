use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::Deserialize;

pub const CONTRACT: u64 = 1;
pub const CONTRACT_VARIABLE: &str = "FORGE_CONTRACT";
pub const COMMANDS_PATH_VARIABLE: &str = "FORGE_COMMANDS_PATH";
pub const DESCRIBE_FLAG: &str = "--forge-describe";
pub const DESCRIBE_TIMEOUT: Duration = Duration::from_secs(2);
const PREFIX: &str = "forge-";
const POLL_INTERVAL: Duration = Duration::from_millis(10);

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error(
        "unknown command `{0}`: it is no built-in, and no forge-{0} is in FORGE_COMMANDS_PATH or PATH"
    )]
    Unknown(String),
    #[error(
        "invalid command name `{0}`: a command name has letters, digits, `-` and `_`, and starts with a letter or digit"
    )]
    InvalidName(String),
    #[error("`{path} {DESCRIBE_FLAG}` did not answer within {}s and was stopped", timeout.as_secs_f32())]
    DescribeTimedOut { path: PathBuf, timeout: Duration },
    #[error("cannot run {path}: {source}")]
    Spawn {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("`{path} {DESCRIBE_FLAG}` failed ({status})")]
    DescribeFailed { path: PathBuf, status: ExitStatus },
    #[error("`{path} {DESCRIBE_FLAG}` printed no valid description: {source}")]
    InvalidDescription {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error(
        "{path} follows forge contract {found}, and this forge follows contract {CONTRACT}: update the command or forge"
    )]
    Contract { path: PathBuf, found: u64 },
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Version {
    contract: u64,
}

#[derive(Debug, Deserialize)]
pub struct Description {
    pub contract: u64,
    pub name: String,
    pub summary: String,
}

#[must_use]
pub fn search_dirs() -> Vec<PathBuf> {
    [COMMANDS_PATH_VARIABLE, "PATH"]
        .iter()
        .filter_map(env::var_os)
        .flat_map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .filter(|dir| !dir.as_os_str().is_empty())
        .collect()
}

pub fn find(name: &str, dirs: &[PathBuf]) -> Result<Plugin, PluginError> {
    if !is_valid_name(name) {
        return Err(PluginError::InvalidName(name.to_owned()));
    }
    dirs.iter()
        .map(|dir| dir.join(format!("{PREFIX}{name}")))
        .find(|path| is_executable(path))
        .map(|path| Plugin {
            name: name.to_owned(),
            path,
        })
        .ok_or_else(|| PluginError::Unknown(name.to_owned()))
}

fn is_valid_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && characters
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

#[must_use]
pub fn list(dirs: &[PathBuf], builtins: &[String]) -> Vec<Plugin> {
    let mut plugins = BTreeMap::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let Some(name) = file_name
                .to_str()
                .and_then(|name| name.strip_prefix(PREFIX))
            else {
                continue;
            };
            if !is_valid_name(name) || builtins.iter().any(|builtin| builtin == name) {
                continue;
            }
            let path = entry.path();
            if plugins.contains_key(name) || !is_executable(&path) {
                continue;
            }
            plugins.insert(
                name.to_owned(),
                Plugin {
                    name: name.to_owned(),
                    path,
                },
            );
        }
    }
    plugins.into_values().collect()
}

fn describe(plugin: &Plugin, timeout: Duration) -> Result<Description, PluginError> {
    let spawn_error = |source| PluginError::Spawn {
        path: plugin.path.clone(),
        source,
    };
    let mut child = Command::new(&plugin.path)
        .arg(DESCRIBE_FLAG)
        .env(CONTRACT_VARIABLE, CONTRACT.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(spawn_error)?;

    let mut stdout = child.stdout.take();
    let reader = thread::spawn(move || {
        let mut output = Vec::new();
        if let Some(stdout) = stdout.as_mut() {
            stdout.read_to_end(&mut output)?;
        }
        Ok::<_, io::Error>(output)
    });

    let Some(status) = wait_within(&mut child, timeout).map_err(spawn_error)? else {
        return Err(PluginError::DescribeTimedOut {
            path: plugin.path.clone(),
            timeout,
        });
    };
    if !status.success() {
        return Err(PluginError::DescribeFailed {
            path: plugin.path.clone(),
            status,
        });
    }

    let output = reader
        .join()
        .unwrap_or_else(|_| Err(io::Error::other("the reader of the description panicked")))
        .map_err(spawn_error)?;
    parse_description(&plugin.path, &output)
}

fn wait_within(child: &mut Child, timeout: Duration) -> io::Result<Option<ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(Some(status));
        }
        if Instant::now() >= deadline {
            child.kill()?;
            child.wait()?;
            return Ok(None);
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn parse_description(path: &Path, stdout: &[u8]) -> Result<Description, PluginError> {
    let invalid = |source| PluginError::InvalidDescription {
        path: path.to_path_buf(),
        source,
    };
    let version: Version = serde_json::from_slice(stdout).map_err(invalid)?;
    if version.contract != CONTRACT {
        return Err(PluginError::Contract {
            path: path.to_path_buf(),
            found: version.contract,
        });
    }
    serde_json::from_slice(stdout).map_err(invalid)
}

#[must_use]
pub fn exec(plugin: &Plugin, args: &[OsString]) -> PluginError {
    if let Err(error) = describe(plugin, DESCRIBE_TIMEOUT) {
        return error;
    }
    let source = Command::new(&plugin.path)
        .args(args)
        .env(CONTRACT_VARIABLE, CONTRACT.to_string())
        .exec();
    PluginError::Spawn {
        path: plugin.path.clone(),
        source,
    }
}

#[must_use]
pub fn overview(dirs: &[PathBuf], builtins: &[String]) -> String {
    let plugins = list(dirs, builtins);
    if plugins.is_empty() {
        return format!(
            "External commands: none; a forge-<name> program in {COMMANDS_PATH_VARIABLE} or PATH runs as `forge <name>`."
        );
    }

    let width = plugins
        .iter()
        .map(|plugin| plugin.name.len())
        .max()
        .unwrap_or(0);
    let lines: Vec<String> = plugins
        .iter()
        .map(|plugin| {
            let summary = describe(plugin, DESCRIBE_TIMEOUT).map_or_else(
                |error| format!("unusable: {error}"),
                |description| description.summary,
            );
            format!("  {:width$}  {summary}", plugin.name)
        })
        .collect();
    format!("External commands:\n{}", lines.join("\n"))
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn another_contract_is_refused_naming_both_versions() {
        let error = parse_description(Path::new("forge-x"), br#"{"contract":2}"#).unwrap_err();
        assert_eq!(
            error.to_string(),
            "forge-x follows forge contract 2, and this forge follows contract 1: update the command or forge"
        );
    }

    #[test]
    fn a_description_without_summary_is_invalid() {
        let error =
            parse_description(Path::new("forge-x"), br#"{"contract":1,"name":"x"}"#).unwrap_err();
        assert!(matches!(error, PluginError::InvalidDescription { .. }));
    }

    fn command(dir: &Path, relative: &str, script: &str) -> PathBuf {
        let path = dir.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("#!/bin/sh\n{script}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn a_name_with_a_path_is_refused_before_any_search() {
        let dir = tempfile::tempdir().unwrap();
        command(dir.path(), "forge-sub/evil", "exit 0");
        command(dir.path(), "y", "exit 0");
        let dirs = [dir.path().join("x")];

        for name in ["sub/evil", "x/../../y", "-flag", ""] {
            let error = find(name, &[dir.path().to_path_buf()]).unwrap_err();
            assert!(
                matches!(error, PluginError::InvalidName(_)),
                "{name}: {error}"
            );
            assert!(matches!(
                find(name, &dirs).unwrap_err(),
                PluginError::InvalidName(_)
            ));
        }
    }

    #[test]
    fn a_valid_name_finds_its_command() {
        let dir = tempfile::tempdir().unwrap();
        let path = command(dir.path(), "forge-hello_2", "exit 0");

        let plugin = find("hello_2", &[dir.path().to_path_buf()]).unwrap();

        assert_eq!(plugin.path, path);
    }

    #[test]
    fn a_describe_that_never_answers_is_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let plugin = Plugin {
            name: "sleepy".to_owned(),
            path: command(dir.path(), "forge-sleepy", "exec sleep 30"),
        };
        let started = Instant::now();

        let error = describe(&plugin, Duration::from_millis(200)).unwrap_err();

        assert!(
            matches!(error, PluginError::DescribeTimedOut { .. }),
            "{error}"
        );
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
