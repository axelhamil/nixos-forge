use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU32, Ordering};

use super::{Commands, Invocation};

static NEXT_MOUNT: AtomicU32 = AtomicU32::new(0);

pub trait Mounts {
    fn mount_read_only(&self, device: &str, options: &str) -> io::Result<ReadOnlyMount<'_>>;
}

type Release<'a> = Box<dyn FnOnce(&Path) + 'a>;

pub struct ReadOnlyMount<'a> {
    path: PathBuf,
    release: Option<Release<'a>>,
}

impl<'a> ReadOnlyMount<'a> {
    #[must_use]
    pub fn new(path: PathBuf, release: impl FnOnce(&Path) + 'a) -> Self {
        Self {
            path,
            release: Some(Box::new(release)),
        }
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ReadOnlyMount<'_> {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            release(&self.path);
        }
    }
}

pub struct TempMounts<'a> {
    commands: &'a dyn Commands,
    root: PathBuf,
}

impl<'a> TempMounts<'a> {
    #[must_use]
    pub fn new(commands: &'a dyn Commands) -> Self {
        Self::under(commands, env::temp_dir())
    }

    #[must_use]
    pub fn under(commands: &'a dyn Commands, root: PathBuf) -> Self {
        Self { commands, root }
    }

    fn unique_dir(&self) -> io::Result<PathBuf> {
        loop {
            let number = NEXT_MOUNT.fetch_add(1, Ordering::Relaxed);
            let dir = self
                .root
                .join(format!("forge-mount-{}-{number}", process::id()));
            match fs::create_dir(&dir) {
                Ok(()) => return Ok(dir),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
    }
}

impl Mounts for TempMounts<'_> {
    fn mount_read_only(&self, device: &str, options: &str) -> io::Result<ReadOnlyMount<'_>> {
        let dir = self.unique_dir()?;
        let target = dir.display().to_string();
        let mounted = self
            .commands
            .run(&Invocation::new("mount", &["-o", options, device, &target]));
        if !mounted.success {
            let _ = fs::remove_dir(&dir);
            return Err(io::Error::other(format!(
                "cannot mount {device}: {}",
                mounted.stderr.trim()
            )));
        }
        let commands = self.commands;
        Ok(ReadOnlyMount::new(dir, move |path| {
            let target = path.display().to_string();
            commands.run(&Invocation::new("umount", &[&target]));
            let _ = fs::remove_dir(path);
        }))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::system::Answer;

    struct Mounter {
        mount_fails: bool,
        ran: RefCell<Vec<String>>,
    }

    impl Commands for Mounter {
        fn run(&self, invocation: &Invocation) -> Answer {
            self.ran.borrow_mut().push(invocation.command_line());
            if invocation.program == "mount" && self.mount_fails {
                return Answer::failed("mount: wrong fs type\n");
            }
            Answer::ok("")
        }
    }

    fn mounter(mount_fails: bool) -> Mounter {
        Mounter {
            mount_fails,
            ran: RefCell::new(Vec::new()),
        }
    }

    #[test]
    fn the_read_only_mount_is_released_when_the_copy_fails() {
        let root = tempfile::tempdir().unwrap();
        let commands = mounter(false);
        let mounts = TempMounts::under(&commands, root.path().to_path_buf());

        let copied: io::Result<()> = (|| {
            let mount = mounts.mount_read_only("/dev/vdb1", "ro,nodev")?;
            assert!(mount.path().is_dir());
            Err(io::Error::other("disk full"))
        })();

        assert!(copied.is_err());
        let ran = commands.ran.borrow();
        assert_eq!(ran.len(), 2);
        assert!(ran[0].starts_with("mount -o ro,nodev /dev/vdb1 "));
        assert!(ran[1].starts_with("umount "));
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn a_failed_mount_removes_its_directory_and_unmounts_nothing() {
        let root = tempfile::tempdir().unwrap();
        let commands = mounter(true);
        let mounts = TempMounts::under(&commands, root.path().to_path_buf());

        let error = mounts.mount_read_only("/dev/vdb1", "ro").err().unwrap();

        assert_eq!(
            error.to_string(),
            "cannot mount /dev/vdb1: mount: wrong fs type"
        );
        assert_eq!(commands.ran.borrow().len(), 1);
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }

    #[test]
    fn two_mounts_get_two_directories() {
        let root = tempfile::tempdir().unwrap();
        let commands = mounter(false);
        let mounts = TempMounts::under(&commands, root.path().to_path_buf());

        let first = mounts.mount_read_only("/dev/vdb1", "ro").unwrap();
        let second = mounts.mount_read_only("/dev/vdc1", "ro").unwrap();

        assert_ne!(first.path(), second.path());
    }
}
