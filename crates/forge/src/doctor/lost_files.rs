use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

const STORE_TARGETS: [&str; 2] = ["/etc/static/", "/nix/store/"];

pub(super) struct Walk<'a> {
    pub root: &'a Path,
    pub pruned: &'a HashSet<PathBuf>,
    pub owned: &'a HashSet<PathBuf>,
    pub since_micros: i64,
}

impl Walk<'_> {
    pub(super) fn lost_files(&self) -> Vec<PathBuf> {
        let since = i128::from(self.since_micros) * 1000;
        let mut lost: Vec<PathBuf> = WalkDir::new(self.root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                entry.depth() == 0 || !self.pruned.contains(&self.shown(entry.path()))
            })
            .filter_map(Result::ok)
            .filter(|entry| !entry.file_type().is_dir())
            .filter(|entry| {
                entry.metadata().is_ok_and(|metadata| {
                    nanos(metadata.mtime(), metadata.mtime_nsec()) > since
                        || nanos(metadata.ctime(), metadata.ctime_nsec()) > since
                })
            })
            .filter(|entry| !entry.path_is_symlink() || !points_into_store(entry.path()))
            .map(|entry| self.shown(entry.path()))
            .filter(|path| !self.owned.contains(path))
            .collect();
        lost.sort();
        lost
    }

    fn shown(&self, path: &Path) -> PathBuf {
        let relative = path.strip_prefix(self.root).unwrap_or(path);
        Path::new("/").join(relative)
    }
}

fn nanos(seconds: i64, nanoseconds: i64) -> i128 {
    i128::from(seconds) * 1_000_000_000 + i128::from(nanoseconds)
}

fn points_into_store(link: &Path) -> bool {
    fs::read_link(link).is_ok_and(|target| {
        let target = target.to_string_lossy();
        STORE_TARGETS
            .iter()
            .any(|prefix| target.starts_with(prefix))
    })
}
