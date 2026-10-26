use super::{Commands, Invocation};

#[must_use]
pub fn entry_number(commands: &dyn Commands, label: &str) -> Option<String> {
    let listing = commands.run(&Invocation::new("efibootmgr", &[]));
    listing
        .stdout
        .lines()
        .filter_map(boot_entry)
        .find(|(_, name)| *name == label)
        .map(|(number, _)| number.to_owned())
}

#[must_use]
pub fn boot_order(commands: &dyn Commands) -> Vec<String> {
    let listing = commands.run(&Invocation::new("efibootmgr", &[]));
    listing
        .stdout
        .lines()
        .find_map(|line| line.strip_prefix("BootOrder: "))
        .unwrap_or_default()
        .split(',')
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

fn boot_entry(line: &str) -> Option<(&str, &str)> {
    let head = line.split('\t').next()?;
    let rest = head.strip_prefix("Boot")?;
    let number = rest.get(..4)?;
    if !number.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let flag_and_label = rest.get(4..)?;
    let label = flag_and_label
        .strip_prefix('*')
        .or_else(|| flag_and_label.strip_prefix(' '))?
        .strip_prefix(' ')?;
    Some((number, label))
}
