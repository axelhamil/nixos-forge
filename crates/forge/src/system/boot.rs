use std::path::Path;

use serde_json::Value;

use super::{Commands, Files, Invocation};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootEntry {
    pub generation: u64,
    pub path: String,
}

impl BootEntry {
    #[must_use]
    pub fn spent_its_tries(&self) -> bool {
        let Some(stem) = self
            .path
            .strip_suffix(".conf")
            .or_else(|| self.path.strip_suffix(".efi"))
        else {
            return false;
        };
        let Some((_, tries_done)) = stem.rsplit_once("+0-") else {
            return false;
        };
        !tries_done.is_empty() && tries_done.bytes().all(|byte| byte.is_ascii_digit())
    }
}

#[must_use]
pub fn boot_entries(commands: &dyn Commands) -> Vec<BootEntry> {
    let answer = commands.run(&Invocation::new(
        "bootctl",
        &["list", "--json=short", "--no-pager"],
    ));
    parse(&answer.stdout)
}

#[must_use]
pub fn generation_went_back(
    files: &dyn Files,
    entry: &BootEntry,
    profiles: &Path,
    current_system: &Path,
) -> bool {
    let link = profiles.join(format!("system-{}-link", entry.generation));
    entry.spent_its_tries() && files.canonicalize(&link) != files.canonicalize(current_system)
}

fn parse(json: &str) -> Vec<BootEntry> {
    let Ok(entries) = serde_json::from_str::<Vec<Value>>(json) else {
        return Vec::new();
    };
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
            Some(BootEntry { generation, path })
        })
        .collect()
}

fn generation_number(version: &str) -> Option<u64> {
    const WORD: &str = "Generation ";
    version.match_indices(WORD).find_map(|(start, _)| {
        let before = version[..start].chars().next_back();
        if before.is_some_and(is_word) {
            return None;
        }
        let rest = &version[start + WORD.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let after = rest[digits.len()..].chars().next();
        if after.is_some_and(is_word) {
            return None;
        }
        digits.parse().ok()
    })
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> BootEntry {
        BootEntry {
            generation: 2,
            path: path.to_owned(),
        }
    }

    #[test]
    fn only_a_counter_at_zero_tries_left_is_spent() {
        assert!(entry("/boot/loader/entries/nixos-generation-2+0-3.conf").spent_its_tries());
        assert!(entry("/boot/EFI/Linux/nixos-generation-2+0-3.efi").spent_its_tries());
        assert!(!entry("/boot/loader/entries/nixos-generation-2+1-2.conf").spent_its_tries());
        assert!(!entry("/boot/loader/entries/nixos-generation-2.conf").spent_its_tries());
        assert!(!entry("/boot/loader/entries/nixos-generation-2+0-.conf").spent_its_tries());
    }

    #[test]
    fn an_entry_names_its_generation_as_a_whole_word() {
        let json = r#"[
            {"version": "Generation 12 NixOS 26.05, Linux Kernel 6.12", "path": "/boot/a.conf"},
            {"version": "Generation 3a", "path": "/boot/b.conf"},
            {"version": "XGeneration 4", "path": "/boot/c.conf"},
            {"version": "Windows Boot Manager", "path": "/boot/d.efi"},
            {"version": "Generation 5", "path": null}
        ]"#;

        assert_eq!(
            parse(json),
            vec![
                BootEntry {
                    generation: 12,
                    path: "/boot/a.conf".to_owned()
                },
                BootEntry {
                    generation: 5,
                    path: "null".to_owned()
                },
            ]
        );
    }
}
