#![allow(dead_code, clippy::unwrap_used, clippy::panic)]

use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_program(name: &str) -> PathBuf {
    let path = env::var_os("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| panic!("{name} is not on PATH"))
}

pub fn write_executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

pub fn sandboxed(program: &Path, dir: &Path, bin: &Path, home: &Path) -> Command {
    let mut command = Command::new(program);
    command
        .current_dir(dir)
        .env_clear()
        .env("PATH", bin)
        .env("HOME", home)
        .env("GIT_CONFIG_NOSYSTEM", "1");
    command
}

pub fn surface(program: &str, args: &[&str], envs: &[(&str, &str)]) -> String {
    let dir = tempfile::tempdir().unwrap();
    let tmp = dir.path().display().to_string();
    let bin = dir.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(dir.path())
        .env_clear()
        .env("PATH", &bin);
    for (name, value) in envs {
        command.env(name, value.replace("{TMP}", &tmp));
    }
    let output = command.output().unwrap();
    let code = output
        .status
        .code()
        .map_or_else(|| "signal".to_owned(), |code| code.to_string());
    format!(
        "exit: {code}\n--- stdout\n{}--- stderr\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .replace(&tmp, "[TMP]")
}
