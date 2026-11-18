use jiff::Timestamp;
use jiff::tz::TimeZone;

pub trait Clock {
    fn now(&self) -> Timestamp;
    fn zone(&self) -> TimeZone;
}
