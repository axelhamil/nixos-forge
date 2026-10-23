use std::io;
use std::path::Path;

use serde_json::Value;

use super::report::{Report, Status};
use super::{Context, Invocation, ROOT_HINT};

const CHECK: &str = "Boot";

pub(super) fn check(context: &Context, report: &mut Report) -> io::Result<()> {
    if !context.root {
        return report.line(Status::Skip, CHECK, &format!("needs root, {ROOT_HINT}"));
    }

    let answer = context.system.commands.run(&Invocation::new(
        "bootctl",
        &["list", "--json=short", "--no-pager"],
    ));
    let Some((generation, path)) = newest_generation(&answer.stdout) else {
        return report.line(
            Status::Skip,
            CHECK,
            "no systemd-boot entries on this machine",
        );
    };

    let files = context.system.files;
    let link = format!("/nix/var/nix/profiles/system-{generation}-link");
    let went_back = spent_its_tries(&path)
        && files.canonicalize(Path::new(&link))
            != files.canonicalize(Path::new("/run/current-system"));
    if !went_back {
        return report.line(
            Status::Ok,
            CHECK,
            "the newest generation started, or has tries left",
        );
    }

    let mut hint = "journalctl -b -1 shows why".to_owned();
    if context.config.auto_upgrade_state.is_some() {
        hint.push_str(", forge doctor --history lists the upgrades");
    }
    report.hinted(
        Status::Warn,
        CHECK,
        "the last update did not start, this machine went back to the previous version",
        &hint,
    )
}

fn newest_generation(json: &str) -> Option<(u64, String)> {
    let entries: Vec<Value> = serde_json::from_str(json).ok()?;
    entries
        .iter()
        .filter_map(|entry| {
            let version = entry.get("version").and_then(Value::as_str)?;
            let generation = generation_number(version)?;
            let path = match entry.get("path") {
                Some(Value::String(path)) => path.clone(),
                Some(other) => other.to_string(),
                None => "null".to_owned(),
            };
            Some((generation, path))
        })
        .max_by_key(|(generation, _)| *generation)
}

fn generation_number(version: &str) -> Option<u64> {
    version
        .match_indices("Generation ")
        .find_map(|(start, word)| {
            let rest = &version[start + word.len()..];
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
}

fn spent_its_tries(path: &str) -> bool {
    let Some(stem) = path
        .strip_suffix(".conf")
        .or_else(|| path.strip_suffix(".efi"))
    else {
        return false;
    };
    let Some((_, tries_done)) = stem.rsplit_once("+0-") else {
        return false;
    };
    !tries_done.is_empty() && tries_done.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_counter_at_zero_tries_left_is_spent() {
        assert!(spent_its_tries(
            "/boot/loader/entries/nixos-generation-2+0-3.conf"
        ));
        assert!(spent_its_tries(
            "/boot/EFI/Linux/nixos-generation-2+0-3.efi"
        ));
        assert!(!spent_its_tries(
            "/boot/loader/entries/nixos-generation-2+1-2.conf"
        ));
        assert!(!spent_its_tries(
            "/boot/loader/entries/nixos-generation-2.conf"
        ));
        assert!(!spent_its_tries(
            "/boot/loader/entries/nixos-generation-2+0-.conf"
        ));
    }
}
