use std::env;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[must_use]
pub fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[must_use]
pub fn find_in_path(program: &str, search_path: &str) -> Option<PathBuf> {
    env::split_paths(search_path)
        .map(|dir| dir.join(program))
        .find(|candidate| is_executable(candidate))
}
