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
