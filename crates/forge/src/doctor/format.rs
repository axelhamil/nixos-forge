use jiff::Timestamp;
use jiff::tz::TimeZone;

const IEC_UNITS: [&str; 8] = ["K", "M", "G", "T", "P", "E", "Z", "Y"];

#[must_use]
pub fn duration(seconds: i64) -> String {
    if seconds < 3600 {
        return format!("{} min", seconds / 60);
    }
    if seconds < 172_800 {
        return format!("{} h", seconds / 3600);
    }
    format!("{} days", seconds / 86400)
}

#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn iec(bytes: u64) -> String {
    if bytes < 1024 {
        return bytes.to_string();
    }

    let mut value = bytes as f64;
    let mut power = 0;
    while value >= 1024.0 && power < IEC_UNITS.len() {
        value /= 1024.0;
        power += 1;
    }

    let decimals = if value < 10.0 { 10.0 } else { 1.0 };
    value = (value * decimals).ceil() / decimals;
    if value >= 1024.0 && power < IEC_UNITS.len() {
        value /= 1024.0;
        power += 1;
    }

    let unit = IEC_UNITS[power.saturating_sub(1)];
    if value < 10.0 {
        return format!("{value:.1}{unit}");
    }
    format!("{value:.0}{unit}")
}

#[must_use]
pub fn short_date(timestamp: Timestamp, zone: &TimeZone) -> String {
    timestamp
        .to_zoned(zone.clone())
        .strftime("%Y-%m-%d %H:%M")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iec_rounds_up_as_numfmt_does() {
        let cases = [
            (0, "0"),
            (1023, "1023"),
            (1024, "1.0K"),
            (1025, "1.1K"),
            (1536, "1.5K"),
            (10239, "10K"),
            (10240, "10K"),
            (10241, "11K"),
            (1_048_575, "1.0M"),
            (536_870_912, "512M"),
            (999_999_999, "954M"),
            (1_073_741_823, "1.0G"),
            (1_610_612_736, "1.5G"),
            (10_737_418_239, "10G"),
            (408_021_893_120, "380G"),
            (1_099_511_627_776, "1.0T"),
        ];
        for (bytes, expected) in cases {
            assert_eq!(iec(bytes), expected, "{bytes}");
        }
    }

    #[test]
    fn duration_picks_minutes_hours_then_days() {
        assert_eq!(duration(59), "0 min");
        assert_eq!(duration(3599), "59 min");
        assert_eq!(duration(3600), "1 h");
        assert_eq!(duration(172_799), "47 h");
        assert_eq!(duration(172_800), "2 days");
    }
}
