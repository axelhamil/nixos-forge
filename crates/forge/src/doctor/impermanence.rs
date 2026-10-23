use std::collections::{BTreeMap, HashSet};
use std::io;
use std::path::{Component, Path, PathBuf};

use super::lost_files::Walk;
use super::report::{Report, Status};
use super::{Context, Mount, boot_setup};

const CHECK: &str = "Impermanence";
const SUMMARY_ROWS: usize = 10;
const ACTIVATION_FILES: [&str; 10] = [
    "/etc/.clean",
    "/etc/.pwd.lock",
    "/etc/.updated",
    "/var/.updated",
    "/etc/NIXOS",
    "/etc/passwd",
    "/etc/group",
    "/etc/shadow",
    "/etc/subuid",
    "/etc/subgid",
];

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    if !context.config.wiped_root {
        return report.line(Status::Skip, CHECK, "/ is not wiped at boot on this host");
    }

    let since_micros = boot_setup::end(context);
    let scope = if context.root {
        ""
    } else {
        ", without the directories only root reads (run sudo forge doctor)"
    };

    let mounts = match context.system.host.mounts() {
        Ok(mounts) => mounts,
        Err(error) => {
            return report.line(
                Status::Warn,
                CHECK,
                &format!("cannot read the mount table: {error}"),
            );
        }
    };
    let pruned = pruned(context, mounts);
    let owned = activation_owned(context);
    let files = Walk {
        root: &context.system.root,
        pruned: &pruned,
        owned: &owned,
        since_micros,
    }
    .lost_files();

    if files.is_empty() {
        return report.line(
            Status::Ok,
            CHECK,
            &format!("nothing written to / since boot is lost at the next boot{scope}"),
        );
    }

    report.hinted(
        Status::Warn,
        CHECK,
        &format!(
            "{} files written to / since boot are lost at the next boot{scope}",
            files.len()
        ),
        "keep them in environment.persistence.\"/persist\", or list them in doctor.ephemeralPaths",
    )?;

    if context.all {
        for file in &files {
            report.detail(&file.display().to_string())?;
        }
        return Ok(());
    }

    for line in summary(&files) {
        report.detail(&line)?;
    }
    report.detail("forge doctor --all lists every file")
}

fn pruned(context: &Context, mounts: Vec<Mount>) -> HashSet<PathBuf> {
    let root = Path::new("/");
    mounts
        .into_iter()
        .map(|mount| mount.target)
        .filter(|target| target != root)
        .chain(context.config.ephemeral_paths.iter().cloned())
        .collect()
}

fn activation_owned(context: &Context) -> HashSet<PathBuf> {
    let clean = context
        .system
        .files
        .read(Path::new("/etc/.clean"))
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_default();

    clean
        .lines()
        .map(|line| PathBuf::from(format!("/etc/{line}")))
        .chain(ACTIVATION_FILES.iter().map(PathBuf::from))
        .collect()
}

fn summary(files: &[PathBuf]) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        *counts.entry(directory(file)).or_default() += 1;
    }

    let mut lines: Vec<(usize, String)> = counts
        .into_iter()
        .map(|(dir, count)| (count, format!("{count:>6}  {dir}")))
        .collect();
    lines.sort_by(|left, right| right.cmp(left));
    lines
        .into_iter()
        .take(SUMMARY_ROWS)
        .map(|(_, line)| line)
        .collect()
}

fn directory(file: &Path) -> String {
    let names: Vec<String> = file
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    let depth = names.len().saturating_sub(1).min(3);
    if depth == 0 {
        return "/".to_owned();
    }
    format!("/{}", names[..depth].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_counts_in_its_directory_up_to_three_levels() {
        assert_eq!(directory(Path::new("/doctor-lost")), "/");
        assert_eq!(directory(Path::new("/srv/doctor/state")), "/srv/doctor");
        assert_eq!(directory(Path::new("/a/b/c/d/e/f")), "/a/b/c");
    }

    #[test]
    fn the_summary_sorts_by_count_then_by_name_descending() {
        let files: Vec<PathBuf> = ["/a/x", "/b/x", "/b/y", "/c/x", "/top"]
            .iter()
            .map(PathBuf::from)
            .collect();

        assert_eq!(
            summary(&files),
            ["     2  /b", "     1  /c", "     1  /a", "     1  /"]
        );
    }
}
