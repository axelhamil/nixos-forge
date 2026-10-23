use std::io;

use super::Context;
use super::report::{Report, Status};

const CHECK: &str = "Signed deploys";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    if context.config.auto_upgrade_state.is_none() {
        return report.line(Status::Skip, CHECK, "auto-upgrade is off");
    }

    let keys = match context.config.deploy_signers {
        0 => {
            return report.hinted(
                Status::Warn,
                CHECK,
                "auto-upgrade applies any deploy commit its sources serve, signed or not",
                "set the host's signers to the public key of DEPLOY_SIGNING_KEY (update.signers, docs/explanation/updates.md)",
            );
        }
        1 => "1 key".to_owned(),
        count => format!("{count} keys"),
    };
    report.line(
        Status::Ok,
        CHECK,
        &format!("auto-upgrade applies only deploy commits signed by {keys} of update.signers"),
    )
}
