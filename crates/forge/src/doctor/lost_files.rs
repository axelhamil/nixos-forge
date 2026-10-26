use std::collections::HashSet;
use std::fs::{self, DirEntry, ReadDir};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const STORE_TARGETS: [&str; 2] = ["/etc/static/", "/nix/store/"];

pub(super) struct Walk<'a> {
    pub root: &'a Path,
    pub pruned: &'a HashSet<PathBuf>,
    pub owned: &'a HashSet<PathBuf>,
    pub since_micros: i64,
}

impl Walk<'_> {
    pub(super) fn lost_files(&self) -> Vec<PathBuf> {
        self.lost_files_reading(|dir| fs::read_dir(dir))
    }

    fn lost_files_reading(
        &self,
        mut read_dir: impl FnMut(&Path) -> io::Result<ReadDir>,
    ) -> Vec<PathBuf> {
        let since = i128::from(self.since_micros) * 1000;
        let mut lost = Vec::new();
        let mut pending = vec![self.root.to_path_buf()];

        while let Some(dir) = pending.pop() {
            let Ok(entries) = read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let shown = self.shown(&path);
                if self.pruned.contains(&shown) {
                    continue;
                }
                let Ok(file_type) = entry.file_type() else {
                    continue;
                };
                if file_type.is_dir() {
                    pending.push(path);
                    continue;
                }
                if changed_since(&entry, since)
                    && !(file_type.is_symlink() && points_into_store(&path))
                    && !self.owned.contains(&shown)
                {
                    lost.push(shown);
                }
            }
        }

        lost.sort();
        lost
    }

    fn shown(&self, path: &Path) -> PathBuf {
        let relative = path.strip_prefix(self.root).unwrap_or(path);
        Path::new("/").join(relative)
    }
}

fn changed_since(entry: &DirEntry, since: i128) -> bool {
    entry.metadata().is_ok_and(|metadata| {
        nanos(metadata.mtime(), metadata.mtime_nsec()) > since
            || nanos(metadata.ctime(), metadata.ctime_nsec()) > since
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pruned_directory_is_never_opened() {
        let disk = tempfile::TempDir::new().unwrap();
        let root = disk.path();
        fs::create_dir_all(root.join("mnt/windows")).unwrap();
        fs::create_dir_all(root.join("srv")).unwrap();
        fs::write(root.join("mnt/windows/pagefile.sys"), b"").unwrap();
        fs::write(root.join("srv/state"), b"").unwrap();
        let pruned = HashSet::from([PathBuf::from("/mnt/windows")]);
        let owned = HashSet::new();
        let walk = Walk {
            root,
            pruned: &pruned,
            owned: &owned,
            since_micros: 0,
        };
        let mut opened = Vec::new();

        let lost = walk.lost_files_reading(|dir| {
            opened.push(walk.shown(dir));
            fs::read_dir(dir)
        });

        opened.sort();
        assert_eq!(opened, ["/", "/mnt", "/srv"].map(PathBuf::from));
        assert_eq!(lost, [PathBuf::from("/srv/state")]);
    }
}
