// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod error;
pub mod photo_scanner;
pub mod preview;
pub mod repo;
pub mod time;
pub mod video_scanner;

pub use error::Error;
pub use photo_scanner::PhotoScanner;
pub use preview::Previewer;
pub use repo::Repository;
pub use time::Year;
pub use time::YearMonth;
pub use video_scanner::VideoScanner;

/// A typedef of the result returned by many methods.
pub type Result<T, E = Error> = std::result::Result<T, E>;
