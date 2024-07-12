// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod database;
pub mod machine_learning;
pub mod path_encoding;
pub mod people;
pub mod photo;
pub mod time;
pub mod video;
pub mod visual;

pub use photo::model::PictureId;
pub use time::Year;
pub use time::YearMonth;
pub use video::VideoId;
pub use visual::Visual;
pub use visual::VisualId;
