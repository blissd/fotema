// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::Month;
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
