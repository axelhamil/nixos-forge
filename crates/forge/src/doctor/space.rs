use std::io;
use std::path::Path;

use super::Context;
use super::format::iec;
use super::report::{Report, Status};

const CHECK: &str = "Disk space";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let root = Path::new("/");
    let space = match context.system.host.space(root) {
        Ok(space) if space.size > 0 => space,
        Ok(_) => return report.line(Status::Warn, CHECK, "/ reports a size of 0"),
        Err(error) => {
            return report.line(
                Status::Warn,
                CHECK,
                &format!("cannot read the free space of /: {error}"),
            );
        }
    };

    let fstype = context
        .system
        .host
        .mounts()
        .unwrap_or_default()
        .into_iter()
        .rev()
        .find(|mount| mount.target == root)
        .map_or_else(|| "-".to_owned(), |mount| mount.fstype);

    let free = u128::from(space.avail) * 100 / u128::from(space.size);
    let summary = format!("{free} % free on / ({fstype}, {} left)", iec(space.avail));

    if free < 10 {
        return report.hinted(
            Status::Warn,
            CHECK,
            &summary,
            "nh clean all --keep 3, and drop old snapshots in /.snapshots",
        );
    }
    report.line(Status::Ok, CHECK, &summary)
}
