use std::env;
use std::fs;
use std::io;
use std::path::Path;

use super::{Host, Mount, Space, mountinfo};
use crate::system::{self, Machine};

impl Host for Machine {
    fn uid(&self) -> u32 {
        rustix::process::getuid().as_raw()
    }

    fn sudo_user(&self) -> Option<String> {
        env::var("SUDO_USER").ok().filter(|user| !user.is_empty())
    }

    fn space(&self, path: &Path) -> io::Result<Space> {
        system::space(path)
    }

    fn mounts(&self) -> io::Result<Vec<Mount>> {
        fs::read_to_string("/proc/self/mountinfo").map(|text| mountinfo::parse(&text))
    }
}
