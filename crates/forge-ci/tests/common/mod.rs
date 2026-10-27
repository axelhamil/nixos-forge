#![allow(dead_code)]

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
