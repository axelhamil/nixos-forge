use std::ffi::OsString;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use forge_ci::{badges, build, flaky, plan, promote, report, reproducibility};

#[derive(Parser)]
#[command(
    name = "forge-ci",
    about = "The CI of a nixos-forge flake: each subcommand is one step of its workflows",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(
        about = "Plan the CI run: the checks to build, the ones skipped and why, as GITHUB_OUTPUT lines",
        disable_help_flag = true
    )]
    Plan {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, help = "EVENT")]
        args: Vec<OsString>,
    },
    #[command(
        about = "Build one check and write its result, excerpt and evidence",
        disable_help_flag = true
    )]
    Build {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "SYSTEM CHECK OUT-DIR [--base-input INPUT --base FLAKE-REF]"
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Write the report of a CI run from its results, for the run summary or a comment",
        disable_help_flag = true
    )]
    Report {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "[--comment]"
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Move the deploy branch of every host whose checks passed",
        disable_help_flag = true
    )]
    Promote {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "RESULTS-DIR SHA"
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Write, publish or read the badges of the badges branch",
        disable_help_flag = true
    )]
    Badges {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "checks OUT-DIR, reproducibility SUMMARY OUT-DIR, publish DIR or read FILE"
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Build every system twice and compare the builds",
        disable_help_flag = true
    )]
    Reproducibility {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "[HOST]..."
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Run one VM check several times and record each run",
        disable_help_flag = true
    )]
    Flaky {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "SYSTEM CHECK RUNS OUT-DIR"
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Plan the VM checks whose flakiness this tree has not measured yet",
        name = "flaky-plan",
        disable_help_flag = true
    )]
    FlakyPlan {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<OsString>,
    },
    #[command(
        about = "Write the flaky report and the history of the runs",
        name = "flaky-report",
        disable_help_flag = true
    )]
    FlakyReport {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "RESULTS-DIR HISTORY-OUT-DIR"
        )]
        args: Vec<OsString>,
    },
}

fn main() -> ExitCode {
    let code = match Cli::parse().command {
        Command::Plan { args } => plan::run(&args),
        Command::Build { args } => build::run(&args),
        Command::Report { args } => report::run(&args),
        Command::Promote { args } => promote::run(&args),
        Command::Badges { args } => badges::run(&args),
        Command::Reproducibility { args } => reproducibility::run(&args),
        Command::Flaky { args } => flaky::run::run(&args),
        Command::FlakyPlan { args } => flaky::plan::run(&args),
        Command::FlakyReport { args } => flaky::report::run(&args),
    };
    ExitCode::from(code)
}
