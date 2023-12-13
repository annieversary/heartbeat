use chrono::{DateTime, Days, Timelike, Utc};

pub fn format_relative(secs: i64) -> String {
    if secs == 0 {
        return "just now".into();
    }

    // numbers taken from https://docs.rs/humantime/latest/src/humantime/duration.rs.html#297

    let mut s = String::new();

    let years = secs / 31_557_600; // 365.25d
    let ydays = secs % 31_557_600;
    let months = ydays / 2_630_016; // 30.44d
    let mdays = ydays % 2_630_016;
    let days = mdays / 86400;
    let day_secs = mdays % 86400;
    let hours = day_secs / 3600;
    let minutes = day_secs % 3600 / 60;
    let seconds = day_secs % 60;

    macro_rules! bweh {
        ($name:expr, $dis:literal, $plural:expr) => {
            if $name > 0 {
                s.push_str(&$name.to_string());
                if $plural {
                    s.push(' ');
                }
                s.push_str($dis);
                if $name > 1 && $plural {
                    s.push('s');
                }
                s.push(' ');
            }
        };
    }

    bweh!(years, "year", true);
    bweh!(months, "month", true);
    bweh!(days, "day", true);
    bweh!(hours, "h", false);
    bweh!(minutes, "m", false);
    bweh!(seconds, "s", false);

    s
}

#[derive(Clone)]
pub struct RangeDays {
    from: DateTime<Utc>,
    to: DateTime<Utc>,
}

impl RangeDays {
    pub fn new(from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        let from = from
            .with_hour(0)
            .and_then(|d| d.with_minute(0))
            .and_then(|d| d.with_second(0))
            .and_then(|d| d.with_nanosecond(0))
            .unwrap();
        let to = to
            .with_hour(0)
            .and_then(|d| d.with_minute(0))
            .and_then(|d| d.with_second(0))
            .and_then(|d| d.with_nanosecond(0))
            .unwrap();
        Self { from, to }
    }
}

impl Iterator for RangeDays {
    type Item = DateTime<Utc>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.from > self.to {
            return None;
        }

        let date = self.from;

        self.from = self.from.checked_add_days(Days::new(1))?;

        Some(date)
    }
}

impl DoubleEndedIterator for RangeDays {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.from > self.to {
            return None;
        }

        let date = self.to;

        self.to = self.to.checked_sub_days(Days::new(1))?;

        Some(date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format() {
        let r = format_relative(10000);
        assert_eq!(r, "2h 46m 40s ");

        let r = format_relative(20000021);
        assert_eq!(r, "7 months 18 days 9h 38m 29s ");

        let r = format_relative(40000021);
        assert_eq!(r, "1 year 3 months 6 days 9h 26m 13s ");

        let r = format_relative(1000000000);
        assert_eq!(r, "31 years 8 months 7 days 19h 17m 52s ");
    }
}
