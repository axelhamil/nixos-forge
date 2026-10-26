use std::io;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::Path;

use rustix::fs::{FileType, Mode, OFlags, Stat, fstat, open, openat};
use rustix::io::Errno;

use super::sources::over_https;
use crate::system::{Commands, Directory, Invocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checkout {
    Absent,
    NoOrigin,
    Origin(String),
    Untrusted,
    Cleartext,
}

pub fn inspect(
    path: &Path,
    directory: &Directory,
    commands: &dyn Commands,
) -> io::Result<Checkout> {
    let folder = match open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(folder) => folder,
        Err(Errno::NOENT | Errno::NOTDIR | Errno::LOOP) => return Ok(Checkout::Absent),
        Err(error) => return Err(error.into()),
    };
    if !trusted(&fstat(&folder)?, directory) {
        return Ok(Checkout::Untrusted);
    }

    let git_dir = match openat(
        &folder,
        ".git",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(git_dir) => git_dir,
        Err(Errno::NOENT) => return Ok(Checkout::NoOrigin),
        Err(Errno::LOOP | Errno::NOTDIR) => return Ok(Checkout::Untrusted),
        Err(error) => return Err(error.into()),
    };
    if !trusted(&fstat(&git_dir)?, directory) {
        return Ok(Checkout::Untrusted);
    }

    let Ok(config) = openat(
        &git_dir,
        "config",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
        Mode::empty(),
    ) else {
        return Ok(Checkout::Untrusted);
    };
    let stat = fstat(&config)?;
    if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile || !trusted(&stat, directory)
    {
        return Ok(Checkout::Untrusted);
    }

    Ok(origin(&config, commands))
}

fn origin(config: &OwnedFd, commands: &dyn Commands) -> Checkout {
    let opened = format!("/proc/{}/fd/{}", std::process::id(), config.as_raw_fd());
    let answer = commands.run(
        &Invocation::new(
            "git",
            &["config", "--file", &opened, "--get", "remote.origin.url"],
        )
        .loud(),
    );
    if !answer.success {
        return Checkout::NoOrigin;
    }

    let url = answer.line();
    if url.starts_with("http://") || url.starts_with("git://") {
        return Checkout::Cleartext;
    }
    Checkout::Origin(over_https(url))
}

fn trusted(stat: &Stat, directory: &Directory) -> bool {
    let group = directory.group_name(stat.st_gid);
    let writers = if matches!(group, Some("wheel" | "root")) {
        0o002
    } else {
        0o022
    };
    let admin = directory.user_name(stat.st_uid).is_some_and(|owner| {
        owner == "root"
            || directory
                .groups_of(owner)
                .is_some_and(|groups| groups.contains(&"wheel"))
    });
    admin && stat.st_mode & writers == 0
}

#[cfg(test)]
mod tests {
    use std::fs::{self, Permissions};
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;
    use crate::system::{Answer, Group, User};

    const OWN_GROUP: &str = "users";

    struct ReadsTheOpenedConfig;

    impl Commands for ReadsTheOpenedConfig {
        fn run(&self, invocation: &Invocation) -> Answer {
            let Ok(text) = fs::read_to_string(&invocation.args[2]) else {
                return Answer::exited(1);
            };
            text.lines()
                .find_map(|line| line.trim().strip_prefix("url = "))
                .map_or_else(|| Answer::exited(1), |url| Answer::ok(&format!("{url}\n")))
        }
    }

    struct Tree {
        dir: TempDir,
    }

    impl Tree {
        fn new(origin: &str) -> Self {
            let dir = tempfile::tempdir().unwrap();
            let git = dir.path().join("nixos/.git");
            fs::create_dir_all(&git).unwrap();
            fs::write(
                git.join("config"),
                format!("[remote \"origin\"]\n\turl = {origin}\n"),
            )
            .unwrap();
            let checkout = Self { dir };
            checkout.chmod("nixos", 0o755);
            checkout.chmod("nixos/.git", 0o755);
            checkout.chmod("nixos/.git/config", 0o644);
            checkout
        }

        fn path(&self, relative: &str) -> PathBuf {
            self.dir.path().join(relative)
        }

        fn chmod(&self, relative: &str, mode: u32) {
            fs::set_permissions(self.path(relative), Permissions::from_mode(mode)).unwrap();
        }

        fn inspect(&self, admin: bool, group: &str) -> Checkout {
            let metadata = fs::metadata(self.dir.path()).unwrap();
            let (uid, gid) = (metadata.uid(), metadata.gid());
            let directory = Directory {
                users: vec![User {
                    name: "owner".to_owned(),
                    uid,
                    gid: u32::MAX,
                }],
                groups: vec![
                    Group {
                        name: group.to_owned(),
                        gid,
                        members: Vec::new(),
                    },
                    Group {
                        name: "wheel".to_owned(),
                        gid: u32::MAX - 1,
                        members: if admin {
                            vec!["owner".to_owned()]
                        } else {
                            Vec::new()
                        },
                    },
                ],
            };
            inspect(&self.path("nixos"), &directory, &ReadsTheOpenedConfig).unwrap()
        }
    }

    #[test]
    fn an_admin_checkout_gives_its_origin_over_https() {
        let checkout = Tree::new("git@github.com:alice/nixos.git");

        assert_eq!(
            checkout.inspect(true, OWN_GROUP),
            Checkout::Origin("https://github.com/alice/nixos.git".to_owned())
        );
    }

    #[test]
    fn a_checkout_owned_by_someone_outside_wheel_is_not_trusted() {
        let checkout = Tree::new("https://github.com/alice/nixos.git");

        assert_eq!(checkout.inspect(false, OWN_GROUP), Checkout::Untrusted);
    }

    #[test]
    fn a_symlinked_git_folder_is_not_followed() {
        let checkout = Tree::new("https://github.com/alice/nixos.git");
        fs::rename(checkout.path("nixos/.git"), checkout.path("real.git")).unwrap();
        symlink(checkout.path("real.git"), checkout.path("nixos/.git")).unwrap();

        assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::Untrusted);
    }

    #[test]
    fn a_git_file_is_not_followed() {
        let checkout = Tree::new("https://github.com/alice/nixos.git");
        fs::rename(checkout.path("nixos/.git"), checkout.path("real.git")).unwrap();
        fs::write(checkout.path("nixos/.git"), "gitdir: ../real.git\n").unwrap();

        assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::Untrusted);
    }

    #[test]
    fn a_symlinked_config_is_not_followed() {
        let checkout = Tree::new("https://github.com/alice/nixos.git");
        fs::rename(checkout.path("nixos/.git/config"), checkout.path("config")).unwrap();
        symlink(checkout.path("config"), checkout.path("nixos/.git/config")).unwrap();

        assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::Untrusted);
    }

    #[test]
    fn a_config_writable_by_a_group_outside_root_and_wheel_is_not_trusted() {
        let checkout = Tree::new("https://github.com/alice/nixos.git");
        checkout.chmod("nixos/.git/config", 0o664);

        assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::Untrusted);
        assert_eq!(
            checkout.inspect(true, "wheel"),
            Checkout::Origin("https://github.com/alice/nixos.git".to_owned())
        );
    }

    #[test]
    fn a_world_writable_checkout_is_not_trusted() {
        let checkout = Tree::new("https://github.com/alice/nixos.git");
        checkout.chmod("nixos", 0o757);

        assert_eq!(checkout.inspect(true, "wheel"), Checkout::Untrusted);
    }

    #[test]
    fn a_cleartext_origin_is_refused() {
        for origin in [
            "http://github.com/alice/nixos.git",
            "git://github.com/alice/nixos.git",
        ] {
            let checkout = Tree::new(origin);

            assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::Cleartext);
        }
    }

    #[test]
    fn a_checkout_without_git_folder_or_origin_adds_no_source() {
        let checkout = Tree::new("unused");
        fs::write(checkout.path("nixos/.git/config"), "[core]\n").unwrap();

        assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::NoOrigin);

        fs::remove_dir_all(checkout.path("nixos/.git")).unwrap();
        assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::NoOrigin);

        fs::remove_dir_all(checkout.path("nixos")).unwrap();
        assert_eq!(checkout.inspect(true, OWN_GROUP), Checkout::Absent);
    }
}
