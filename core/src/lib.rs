// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod controller;
pub mod error;
pub mod preview;
pub mod repo;
pub mod scanner;
pub mod time;

pub use controller::Controller;
pub use error::Error;
pub use preview::Previewer;
pub use repo::Repository;
pub use scanner::Scanner;
pub use time::Year;
pub use time::YearMonth;

/// A typedef of the result returned by many methods.
pub type Result<T, E = Error> = std::result::Result<T, E>;
