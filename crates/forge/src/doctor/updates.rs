use std::io;
use std::path::Path;
use std::time::Duration;

use super::report::{Report, Status};
use super::{Answer, Context, Invocation};

const CHECK: &str = "Updates";
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    let Some(flake) = context
        .config
        .update_flake
        .as_deref()
        .filter(|flake| context.system.files.is_dir(&flake.join(".git")))
    else {
        return report.line(
            Status::Skip,
            CHECK,
            "the host's flake is not a git checkout on this machine",
        );
    };
    let shown = flake.display().to_string();

    let fetch = git(context, flake, &["fetch", "--quiet"])
        .timeout(FETCH_TIMEOUT)
        .discard_output();
    let fetched = run(context, &fetch).success;

    let counts = run(
        context,
        &git(
            context,
            flake,
            &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
        ),
    );
    let Some((behind, ahead)) = counts
        .success
        .then(|| parse_counts(&counts.stdout))
        .flatten()
    else {
        return report.hinted(
            Status::Warn,
            CHECK,
            &format!("cannot compare {shown} with its upstream branch"),
            &format!("git -C {shown} branch --set-upstream-to origin/main"),
        );
    };

    let mut state = Vec::new();
    if behind > 0 {
        state.push(format!("behind by {behind}"));
    }
    if ahead > 0 {
        state.push(format!("ahead by {ahead}"));
    }
    if !run(context, &git(context, flake, &["status", "--porcelain"]))
        .stdout
        .trim()
        .is_empty()
    {
        state.push("uncommitted changes".to_owned());
    }
    if !fetched {
        state.push("fetch failed, counts from the last fetch".to_owned());
    }

    let summary = format!("{shown}: {}", state.join(", "));
    if behind > 0 {
        return report.hinted(Status::Warn, CHECK, &summary, "run update");
    }
    if !state.is_empty() {
        return report.hinted(
            Status::Warn,
            CHECK,
            &summary,
            "commit and push, so other machines and CI get it",
        );
    }
    report.line(
        Status::Ok,
        CHECK,
        &format!("{shown} is up to date with its upstream"),
    )
}

fn parse_counts(stdout: &str) -> Option<(u64, u64)> {
    let mut counts = stdout.split_whitespace().map(str::parse::<u64>);
    Some((counts.next()?.ok()?, counts.next()?.ok()?))
}

fn git(context: &Context, flake: &Path, args: &[&str]) -> Invocation {
    let flake_arg = flake.display().to_string();
    let args: Vec<&str> = ["-C", flake_arg.as_str()]
        .into_iter()
        .chain(args.iter().copied())
        .collect();
    let invocation = Invocation::new("git", &args);

    match context.system.files.owner(flake) {
        Some(owner) if context.root && owner.uid != 0 => invocation.run_as(owner),
        _ => invocation,
    }
}

fn run(context: &Context, invocation: &Invocation) -> Answer {
    context.system.commands.run(invocation)
}
