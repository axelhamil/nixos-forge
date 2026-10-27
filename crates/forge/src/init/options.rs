use std::ffi::OsString;

pub const USAGE: &str = "Usage: nix run .#init -- [options]

Writes modules/hosts/<host>/default.nix from the commented example host and
modules/users/<login>/default.nix from the example person, removes the examples and stages
the result. Asks for every answer not given as an option.

  --host NAME            host name: 1 to 63 letters, digits, - and _, starting and ending with a letter or digit
  --system SYSTEM        x86_64-linux (default) or aarch64-linux
  --profile PROFILE      desktop, laptop, server or pi
  --login LOGIN          your login name, and the name of your person in modules/users
  --full-name NAME       your full name
  --email EMAIL          your email, used by git; empty for none
  --time-zone ZONE       such as Europe/Paris (default: this machine's)
  --keyboard LAYOUT      such as fr (default: us)
  --locale LOCALE        such as fr_FR.UTF-8 (default: en_US.UTF-8)
  --features LIST        features to turn on, comma or space separated (default: picked for the profile)
  --hardware LIST        hardware modules or nixos-hardware profiles (default: firmware audio bluetooth for desktop and laptop, raspberry-pi-5 for pi, else none)
  --apps LIST            apps for every user: starter apps or nixpkgs packages (default: picked for the profile)
  --keep-server          keep the example-server host
  --keep-pi              keep the example-pi host
  --yes                  ask nothing: use the defaults, fail on a missing answer
  -h, --help             show this help
";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Flags {
    pub host: String,
    pub system: String,
    pub profile: String,
    pub login: String,
    pub full_name: String,
    pub email: Option<String>,
    pub time_zone: String,
    pub keyboard: String,
    pub locale: String,
    pub features: Option<String>,
    pub hardware: Option<String>,
    pub apps: Option<String>,
    pub keep_server: bool,
    pub keep_pi: bool,
    pub yes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Run(Box<Flags>),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("{0} needs a value")]
    Missing(String),
    #[error("unknown option {0} (see --help)")]
    Unknown(String),
}

const VALUED: [&str; 12] = [
    "--host",
    "--system",
    "--profile",
    "--login",
    "--full-name",
    "--email",
    "--time-zone",
    "--keyboard",
    "--locale",
    "--features",
    "--hardware",
    "--apps",
];

pub fn parse(args: &[OsString]) -> Result<Parsed, ParseError> {
    let mut flags = Flags::default();
    let mut args = args.iter().map(|arg| arg.to_string_lossy().into_owned());
    while let Some(arg) = args.next() {
        let (option, inline) = match arg.split_once('=') {
            Some((option, value)) if option.starts_with("--") => {
                (option.to_owned(), Some(value.to_owned()))
            }
            _ => (arg.clone(), None),
        };
        let value = if VALUED.contains(&option.as_str()) {
            match inline {
                Some(value) => value,
                None => args
                    .next()
                    .ok_or_else(|| ParseError::Missing(option.clone()))?,
            }
        } else {
            String::new()
        };

        match option.as_str() {
            "--host" => flags.host = value,
            "--system" => flags.system = value,
            "--profile" => flags.profile = value,
            "--login" => flags.login = value,
            "--full-name" => flags.full_name = value,
            "--email" => flags.email = Some(value),
            "--time-zone" => flags.time_zone = value,
            "--keyboard" => flags.keyboard = value,
            "--locale" => flags.locale = value,
            "--features" => flags.features = Some(value),
            "--hardware" => flags.hardware = Some(value),
            "--apps" => flags.apps = Some(value),
            "--keep-server" => flags.keep_server = true,
            "--keep-pi" => flags.keep_pi = true,
            "--yes" => flags.yes = true,
            "-h" | "--help" => return Ok(Parsed::Help),
            _ => return Err(ParseError::Unknown(option)),
        }
    }
    Ok(Parsed::Run(Box::new(flags)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: &[&str]) -> Result<Parsed, ParseError> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        parse(&args)
    }

    #[test]
    fn values_come_after_the_option_or_after_an_equal_sign_and_may_be_empty() {
        let Ok(Parsed::Run(flags)) = parsed(&[
            "--host=box",
            "--login",
            "bob",
            "--email=",
            "--features",
            "disk,ssh",
            "--keep-pi",
            "--yes=whatever",
        ]) else {
            panic!("the options were refused");
        };

        assert_eq!(flags.host, "box");
        assert_eq!(flags.login, "bob");
        assert_eq!(flags.email.as_deref(), Some(""));
        assert_eq!(flags.features.as_deref(), Some("disk,ssh"));
        assert_eq!(flags.hardware, None);
        assert!(flags.keep_pi && flags.yes && !flags.keep_server);
    }

    #[test]
    fn a_missing_value_and_an_unknown_option_are_usage_errors() {
        assert_eq!(
            parsed(&["--host"]),
            Err(ParseError::Missing("--host".to_owned()))
        );
        assert_eq!(
            parsed(&["box"]).unwrap_err().to_string(),
            "unknown option box (see --help)"
        );
        assert_eq!(
            parsed(&["-h=1"]),
            Err(ParseError::Unknown("-h=1".to_owned()))
        );
    }

    #[test]
    fn help_stops_the_parsing_where_it_stands() {
        assert_eq!(parsed(&["--yes", "--help", "--bogus"]), Ok(Parsed::Help));
        assert!(parsed(&["--bogus", "-h"]).is_err());
    }
}
