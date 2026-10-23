use std::io;

use super::report::{Report, Status};
use super::{Context, Invocation};

const CHECK: &str = "Failed units";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let system = failed_units(context, &[]);
    let user = if !context.root {
        failed_units(context, &["--user"])
    } else if let Some(sudo_user) = context.system.host.sudo_user() {
        failed_units(context, &["--user", &format!("--machine={sudo_user}@")])
    } else {
        String::new()
    };

    if system.is_empty() && user.is_empty() {
        return report.line(Status::Ok, CHECK, "none");
    }

    let mut scopes = Vec::new();
    if !system.is_empty() {
        scopes.push(format!("system: {system}"));
    }
    if !user.is_empty() {
        scopes.push(format!("user: {user}"));
    }

    report.hinted(
        Status::Fail,
        CHECK,
        &scopes.join("; "),
        "systemctl status <unit>, journalctl -b -u <unit> (--user for user units)",
    )
}

fn failed_units(context: &Context, scope: &[&str]) -> String {
    let args: Vec<&str> = scope
        .iter()
        .copied()
        .chain([
            "list-units",
            "--failed",
            "--plain",
            "--no-legend",
            "--no-pager",
        ])
        .collect();

    context
        .system
        .commands
        .run(&Invocation::new("systemctl", &args))
        .stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .collect::<Vec<_>>()
        .join(" ")
}
