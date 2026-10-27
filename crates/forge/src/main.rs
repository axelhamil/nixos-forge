use std::env;
use std::ffi::OsString;
use std::io::{self, IsTerminal};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use forge::auto_upgrade::{self, Outcome, Overrides, Paths, Ports};
use forge::doctor::{self, Config, Forks, Options, System};
use forge::enroll;
use forge::system::{Git, Machine, config};
use forge::{check_docs, dual_boot, init, install, plugins, update};

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
    #[command(
        about = "Pull the version CI validated for this machine and apply it, as the auto-upgrade service does",
        name = "auto-upgrade"
    )]
    AutoUpgrade,
    #[command(
        about = "Pull the host's flake checkout, then run nh os switch with the given arguments",
        disable_help_flag = true
    )]
    Update {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments for nh os switch"
        )]
        args: Vec<OsString>,
    },
    #[command(about = "Enroll a TPM2 or a security key to unlock the LUKS disk of this host")]
    Enroll {
        #[command(subcommand)]
        method: Enrollment,
    },
    #[command(
        about = "Install this installer's host on a whole disk, or beside Windows, asking for every value not given",
        disable_help_flag = true
    )]
    Install {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "--disk DEVICE, --beside-windows, --passphrase-file FILE, --password-file USER=FILE, --yes, --no-reboot"
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Keep Windows in the boot menu and the firmware boot order, or make room for NixOS beside it",
        name = "dual-boot",
        disable_help_flag = true
    )]
    DualBoot {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "sync, or install-beside-windows DISK SYSTEM"
        )]
        args: Vec<OsString>,
    },
    #[command(
        about = "Make a fresh fork yours: write its host and person from the examples, then remove them",
        disable_help_flag = true
    )]
    Init {
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "--host NAME, --login LOGIN, --yes and the other options of --help"
        )]
        args: Vec<OsString>,
    },
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(Subcommand)]
enum Enrollment {
    #[command(
        about = "Enroll the TPM2 bound to Secure Boot, as the tpm2-enroll service does at boot"
    )]
    Tpm2,
    #[command(about = "Enroll a FIDO2 security key, the passphrase stays as a fallback")]
    Fido2 {
        #[arg(help = "LUKS device, the disk of this host by default")]
        device: Option<String>,
    },
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
        Command::AutoUpgrade => auto_upgrade_command(),
        Command::Update { .. } => update_command(&update::arguments(env::args_os())),
        Command::Enroll { method } => enroll_command(&method),
        Command::Install { .. } => install_command(),
        Command::DualBoot { .. } => dual_boot_command(),
        Command::Init { .. } => init_command(),
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

    let machine = Machine;
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

fn auto_upgrade_command() -> ExitCode {
    let path = env::var_os(auto_upgrade::CONFIG_VARIABLE).map(PathBuf::from);
    let config: auto_upgrade::Config = match config::load(path.as_deref()) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("forge auto-upgrade: {error}");
            return ExitCode::FAILURE;
        }
    };

    let machine = Machine;
    let ports = Ports {
        commands: &machine,
        streams: &machine,
        clock: &machine,
        accounts: &machine,
        files: &machine,
    };
    match auto_upgrade::run(
        &config,
        &ports,
        &Paths::default(),
        &overrides_from_env(),
        &mut io::stdout().lock(),
    ) {
        Ok(Some(Outcome::Failed)) => ExitCode::FAILURE,
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("forge auto-upgrade: {error}");
            ExitCode::FAILURE
        }
    }
}

fn overrides_from_env() -> Overrides {
    let variable = |name| env::var(name).ok().filter(|value| !value.is_empty());
    Overrides {
        current_rev: variable("CURRENT_REV"),
        next_rev: variable("NEXT_REV"),
        next_did_not_start: variable("NEXT_DID_NOT_START"),
    }
}

fn update_command(args: &[OsString]) -> ExitCode {
    let Some(flake) = env::var_os(update::FLAKE_VARIABLE) else {
        eprintln!(
            "forge update: {} is not set: give this host a `flake` field to update from",
            update::FLAKE_VARIABLE
        );
        return ExitCode::FAILURE;
    };
    let plan = update::plan(&flake, Path::is_dir, args);

    if let Some(checkout) = &plan.pull {
        let machine = Machine;
        let pulled =
            Git::new(&machine, &checkout.display().to_string()).attached(&["pull", "--ff-only"]);
        if !pulled.success {
            if !pulled.stderr.is_empty() {
                eprintln!("forge update: {}", pulled.stderr);
            }
            return ExitCode::from(pulled.exit_code());
        }
    }

    let error = update::switch(&plan).exec();
    eprintln!("forge update: cannot run nh: {error}");
    ExitCode::FAILURE
}

fn enroll_command(method: &Enrollment) -> ExitCode {
    let method = match method {
        Enrollment::Tpm2 => enroll::Method::Tpm2,
        Enrollment::Fido2 { device } => enroll::Method::Fido2 {
            device: device.as_deref(),
        },
    };
    let path = env::var_os(enroll::CONFIG_VARIABLE).map(PathBuf::from);
    ExitCode::from(enroll::run(method, path.as_deref()))
}

fn install_command() -> ExitCode {
    let args = update::arguments_after(install::SUBCOMMAND, env::args_os());
    let path = env::var_os(install::CONFIG_VARIABLE).map(PathBuf::from);
    ExitCode::from(install::run(&args, path.as_deref()))
}

fn dual_boot_command() -> ExitCode {
    let args = update::arguments_after(dual_boot::SUBCOMMAND, env::args_os());
    let config_path = |variable: &str| env::var_os(variable).map(PathBuf::from);
    ExitCode::from(dual_boot::run(&args, &config_path))
}

fn init_command() -> ExitCode {
    let args = update::arguments_after(init::SUBCOMMAND, env::args_os());
    ExitCode::from(init::run(&args))
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
