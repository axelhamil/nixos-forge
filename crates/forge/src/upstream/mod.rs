mod notes;
mod sync;

use std::env;
use std::ffi::OsString;
use std::io;

use crate::system::prompt::{Console, Terminal};
use crate::system::{Commands, Machine, Storage, find_in_path};

pub const SUBCOMMAND: &str = "upstream";

const USAGE_ERROR: u8 = 2;

const USAGE: &str = "Usage: forge upstream [--yes] [URL | OWNER/REPO]

Shows what the starter changed since the main branch of origin last merged it: the breaking
changes first, then its CHANGELOG.md. Then offers to merge its latest release tag into a
chore/sync-upstream-<tag> branch made from origin/main, pushes that branch to origin and
opens a pull request against main with gh when it is logged in, or prints the gh command.

The starter is the argument, else the git remote named upstream, else
https://github.com/axelhamil/nixos-forge.git. OWNER/REPO means a GitHub repository.

  --yes        prepare the branch without asking
  -h, --help   show this help
";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    pub yes: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Run(Options),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("unknown option {0} (see --help)")]
    Unknown(String),
    #[error("one starter only, got {0} and {1}")]
    TwoStarters(String, String),
}

pub fn parse(args: &[OsString]) -> Result<Parsed, ParseError> {
    let mut options = Options::default();
    for arg in args.iter().map(|arg| arg.to_string_lossy().into_owned()) {
        match arg.as_str() {
            "--yes" | "-y" => options.yes = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            _ if arg.starts_with('-') => return Err(ParseError::Unknown(arg)),
            _ => {
                if let Some(first) = options.source.filter(|first| !first.is_empty()) {
                    return Err(ParseError::TwoStarters(first, arg));
                }
                options.source = Some(arg);
            }
        }
    }
    options.source = options.source.filter(|source| !source.is_empty());
    Ok(Parsed::Run(options))
}

#[must_use]
pub fn usage() -> &'static str {
    USAGE
}

pub struct Context {
    pub ci: bool,
    pub gh: Option<String>,
}

pub struct Ports<'a> {
    pub commands: &'a dyn Commands,
    pub storage: &'a dyn Storage,
    pub terminal: &'a mut dyn Terminal,
    pub out: &'a mut dyn io::Write,
    pub err: &'a mut dyn io::Write,
}

#[must_use]
pub fn run(args: &[OsString]) -> u8 {
    let options = match parse(args) {
        Ok(Parsed::Run(options)) => options,
        Ok(Parsed::Help) => {
            print!("{USAGE}");
            return 0;
        }
        Err(error) => {
            eprintln!("forge upstream: {error}");
            return USAGE_ERROR;
        }
    };

    let context = Context {
        ci: env::var("GITHUB_ACTIONS").is_ok_and(|value| value == "true"),
        gh: find_in_path("gh", &env::var("PATH").unwrap_or_default())
            .map(|path| path.display().to_string()),
    };
    let mut terminal = Console::stdio(None);
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    let mut ports = Ports {
        commands: &Machine,
        storage: &Machine,
        terminal: &mut terminal,
        out: &mut stdout,
        err: &mut stderr,
    };
    match sync::sync(&options, &context, &mut ports) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("forge upstream: {error}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Result<Parsed, ParseError> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        parse(&args)
    }

    #[test]
    fn yes_in_both_spellings_and_one_starter_are_read() {
        assert_eq!(
            parsed(&["-y", "bob/forge"]),
            Ok(Parsed::Run(Options {
                yes: true,
                source: Some("bob/forge".to_owned()),
            }))
        );
        assert_eq!(
            parsed(&["--yes"]),
            Ok(Parsed::Run(Options {
                yes: true,
                source: None,
            }))
        );
        assert_eq!(parsed(&[]), Ok(Parsed::Run(Options::default())));
    }

    #[test]
    fn an_empty_starter_is_no_starter_as_in_the_shell() {
        assert_eq!(parsed(&[""]), Ok(Parsed::Run(Options::default())));
        assert_eq!(
            parsed(&["", "bob/forge"]),
            Ok(Parsed::Run(Options {
                yes: false,
                source: Some("bob/forge".to_owned()),
            }))
        );
    }

    #[test]
    fn help_wins_over_what_follows_it() {
        assert_eq!(parsed(&["--yes", "-h", "--bogus"]), Ok(Parsed::Help));
        assert_eq!(parsed(&["--help"]), Ok(Parsed::Help));
    }

    #[test]
    fn an_unknown_option_or_a_second_starter_is_refused_with_the_shell_message() {
        assert_eq!(
            parsed(&["--force"]).unwrap_err().to_string(),
            "unknown option --force (see --help)"
        );
        assert_eq!(
            parsed(&["a/b", "c/d"]).unwrap_err().to_string(),
            "one starter only, got a/b and c/d"
        );
    }
}
