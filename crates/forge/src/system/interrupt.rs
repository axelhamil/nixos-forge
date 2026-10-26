use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};

#[derive(Debug, Clone, Default)]
pub struct Flag {
    signal: Arc<AtomicUsize>,
}

impl Flag {
    pub fn register() -> io::Result<Self> {
        let flag = Self::default();
        for signal in [SIGINT, SIGTERM, SIGHUP] {
            let number = usize::try_from(signal).map_err(io::Error::other)?;
            signal_hook::flag::register_usize(signal, Arc::clone(&flag.signal), number)?;
        }
        Ok(flag)
    }

    #[must_use]
    pub fn raised(&self) -> Option<i32> {
        match self.signal.load(Ordering::SeqCst) {
            0 => None,
            signal => i32::try_from(signal).ok(),
        }
    }

    #[cfg(test)]
    pub(crate) fn raised_by(signal: i32) -> Self {
        let number = usize::try_from(signal).unwrap_or_default();
        Self {
            signal: Arc::new(AtomicUsize::new(number)),
        }
    }

    #[cfg(test)]
    pub(crate) fn raise(&self, signal: i32) {
        let number = usize::try_from(signal).unwrap_or_default();
        self.signal.store(number, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_registered_flag_records_the_signal_that_stopped_the_run() {
        let flag = Flag::register().unwrap();
        assert_eq!(flag.raised(), None);

        signal_hook::low_level::raise(SIGHUP).unwrap();

        assert_eq!(flag.raised(), Some(SIGHUP));
    }
}
