use std::io;
use std::path::Path;

use crate::invocation::Invocation;

#[derive(Debug)]
pub struct Streamed {
    pub succeeded: bool,
    pub logged: io::Result<()>,
}

pub trait Streams {
    fn stream(&self, invocation: &Invocation, log: &Path) -> io::Result<Streamed>;
}
