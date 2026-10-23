use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use forge::doctor::{self, Config, Forks, Options, System};
use forge::{check_docs, plugins};

#[derive(Parser)]
#[command(
    name = "forge",
    version,
    about = "The command line of a nixos-forge flake: built-in commands, and the forge-<name> commands a fork adds",
    allow_external_subcommands = true,
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(
        about = "Check the docs: links and anchors, the docs/README.md index, names a fork lacks, a section per module"
    )]
    CheckDocs {
        #[arg(default_value = ".", help = "Root of the flake")]
        root: PathBuf,
    },
    #[command(
        about = "Check this machine, one line per check: OK, WARN, FAIL or SKIP when it does not apply; run it with sudo for the checks that need root"
    )]
    Doctor {
        #[arg(
            long,
            conflicts_with = "history",
            help = "List every file on / that the next boot erases"
        )]
        all: bool,
        #[arg(long, help = "List the last 10 runs of auto-upgrade")]
        history: bool,
    },
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

fn main() -> ExitCode {
    let dirs = plugins::search_dirs();
    let mut command = Cli::command();
    if wants_overview() {
        command = command.after_help(plugins::overview(&dirs, &builtins()));
    }

    let cli = match Cli::from_arg_matches(&command.get_matches()) {
        Ok(cli) => cli,
        Err(error) => error.exit(),
    };

    match cli.command {
        Command::CheckDocs { root } => check_docs_command(&root),
        Command::Doctor { all, history } => doctor_command(&dirs, all, history),
        Command::External(args) => external_command(&dirs, &args),
    }
}

fn builtins() -> Vec<String> {
    Cli::command()
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_owned())
        .chain(["help".to_owned()])
        .collect()
}

fn wants_overview() -> bool {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => true,
        [only] => ["help", "--help", "-h"].iter().any(|flag| only == *flag),
        _ => false,
    }
}

fn check_docs_command(root: &Path) -> ExitCode {
    let outcome = match check_docs::run(root) {
        Ok(outcome) => outcome,
        Err(error) => {
            eprintln!("check-docs: {error}");
            return ExitCode::FAILURE;
        }
    };

    if outcome.passed() {
        println!("{}", outcome.summary());
        return ExitCode::SUCCESS;
    }
    for problem in &outcome.problems {
        eprintln!("{problem}");
    }
    eprintln!("{}", outcome.summary());
    ExitCode::FAILURE
}

fn doctor_command(dirs: &[PathBuf], all: bool, history: bool) -> ExitCode {
    let path = env::var_os(doctor::CONFIG_VARIABLE)
        .map_or_else(|| PathBuf::from(doctor::CONFIG_FILE), PathBuf::from);
    let config = match Config::load(&path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("forge doctor: {error}");
            return ExitCode::FAILURE;
        }
    };

    let machine = doctor::real::Machine;
    let system = System {
        commands: &machine,
        files: &machine,
        host: &machine,
        clock: &machine,
        root: PathBuf::from("/"),
        forks: Forks {
            dirs: dirs.to_vec(),
            builtins: builtins(),
            timeout: doctor::FORK_TIMEOUT,
        },
    };
    let mut out = io::stdout().lock();

    if history {
        return match doctor::history(&config, &system, &mut out) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("forge doctor: cannot write the history: {error}");
                ExitCode::FAILURE
            }
        };
    }

    let options = Options {
        all,
        colors: io::stdout().is_terminal(),
    };
    match doctor::run(&config, &system, options, &mut out) {
        Ok(totals) if totals.failures > 0 => ExitCode::FAILURE,
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("forge doctor: cannot write the report: {error}");
            ExitCode::FAILURE
        }
    }
}

fn external_command(dirs: &[PathBuf], args: &[OsString]) -> ExitCode {
    let Some((name, rest)) = args.split_first() else {
        return ExitCode::FAILURE;
    };
    let name = name.to_string_lossy();
    let error = match plugins::find(&name, dirs) {
        Ok(plugin) => plugins::exec(&plugin, rest),
        Err(error) => error,
    };
    eprintln!("forge: {error}");
    ExitCode::FAILURE
}
