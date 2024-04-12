// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod error;
pub mod photo;
pub mod time;
pub mod video;

pub use error::Error;
pub use time::Year;
pub use time::YearMonth;

/// A typedef of the result returned by many methods.
pub type Result<T, E = Error> = std::result::Result<T, E>;
