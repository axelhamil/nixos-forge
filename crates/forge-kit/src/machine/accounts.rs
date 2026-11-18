use std::fs;

use super::Machine;
use crate::ports::accounts::{Accounts, Directory};

impl Accounts for Machine {
    fn directory(&self) -> Directory {
        let read = |path| fs::read_to_string(path).unwrap_or_default();
        Directory::parse(&read("/etc/passwd"), &read("/etc/group"))
    }
}
