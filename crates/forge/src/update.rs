use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

const SUBCOMMAND: &str = "update";

pub const FLAKE_VARIABLE: &str = "FORGE_UPDATE_FLAKE";
const FRESH_TARBALLS: &str = "tarball-ttl = 0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub pull: Option<PathBuf>,
    pub nix_config: Option<&'static str>,
    pub switch: Vec<OsString>,
}

#[must_use]
pub fn plan(flake: &OsStr, is_dir: impl Fn(&Path) -> bool, args: &[OsString]) -> Plan {
    let checkout = Some(PathBuf::from(flake)).filter(|flake| is_dir(&flake.join(".git")));
    let switch = ["os", "switch"]
        .into_iter()
        .map(OsString::from)
        .chain(args.iter().cloned())
        .collect();
    Plan {
        nix_config: checkout.is_none().then_some(FRESH_TARBALLS),
        pull: checkout,
        switch,
    }
}

#[must_use]
pub fn arguments(raw: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    arguments_after(SUBCOMMAND, raw)
}

#[must_use]
pub fn arguments_after(subcommand: &str, raw: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    raw.into_iter()
        .skip(1)
        .skip_while(|arg| arg != subcommand)
        .skip(1)
        .collect()
}

#[must_use]
pub fn switch(plan: &Plan) -> Command {
    let mut command = Command::new("nh");
    command.args(&plan.switch);
    if let Some(nix_config) = plan.nix_config {
        command.env("NIX_CONFIG", nix_config);
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn a_checkout_is_pulled_before_nh_switches_with_the_arguments() {
        let plan = plan(
            OsStr::new("/home/alice/nixos"),
            |path| path == Path::new("/home/alice/nixos/.git"),
            &args(&["-s", "gaming", "--no-nom"]),
        );

        assert_eq!(plan.pull, Some(PathBuf::from("/home/alice/nixos")));
        assert_eq!(plan.nix_config, None);
        assert_eq!(
            plan.switch,
            args(&["os", "switch", "-s", "gaming", "--no-nom"])
        );
    }

    #[test]
    fn every_argument_after_the_subcommand_is_kept_as_written() {
        let raw = args(&["forge", "update", "--", "--show-trace"]);

        assert_eq!(arguments(raw), args(&["--", "--show-trace"]));
    }

    #[test]
    fn a_flake_reference_is_fetched_again_without_a_pull() {
        let plan = plan(OsStr::new("github:alice/nixos"), |_| false, &[]);

        assert_eq!(plan.pull, None);
        assert_eq!(plan.nix_config, Some("tarball-ttl = 0"));
        assert_eq!(plan.switch, args(&["os", "switch"]));
    }
}
