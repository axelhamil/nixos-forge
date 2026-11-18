use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::Machine;
use crate::ports::clock::Clock;

impl Clock for Machine {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }

    fn zone(&self) -> TimeZone {
        TimeZone::system()
    }
}
