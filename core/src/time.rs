// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{Month, TimeDelta};
use std::fmt::Display;

pub type Year = i32;

#[derive(Debug, PartialOrd, PartialEq)]
pub struct YearMonth {
    pub year: Year,
    pub month: Month,
}

impl Display for YearMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.month.name(), self.year)
    }
}

impl YearMonth {
    pub fn new(year: Year, month: Month) -> YearMonth {
        YearMonth { year, month }
    }
}

pub fn format_hhmmss(delta: &TimeDelta) -> String {
    let total_seconds = delta.num_seconds();
    let seconds = total_seconds % 60;
    let minutes = (total_seconds / 60) % 60;
    let hours = (total_seconds / 60) / 60;
    let hhmmss = if hours == 0 {
        format!("{}:{:0>2}", minutes, seconds)
    } else {
        format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
    };
    hhmmss
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timedelta_format() {
        let one_sec = TimeDelta::try_seconds(1).unwrap();
        assert_eq!("0:01", &format_hhmmss(&one_sec));

        let ten_sec = TimeDelta::try_seconds(10).unwrap();
        assert_eq!("0:10", &format_hhmmss(&ten_sec));

        let one_minute = TimeDelta::try_seconds(60).unwrap();
        assert_eq!("1:00", &format_hhmmss(&one_minute));

        let one_hour = TimeDelta::try_seconds(3600).unwrap();
        assert_eq!("1:00:00", &format_hhmmss(&one_hour));
    }
}
