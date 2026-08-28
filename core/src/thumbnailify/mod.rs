// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod error;
pub mod file;
pub mod hash;
pub mod quality;
pub mod sizes;
pub mod thumbnailer;

pub use error::ThumbnailError;
pub use file::get_file_uri;
pub use file::get_thumbnail_hash_output;
pub use file::get_thumbnail_path;
pub use file::is_failed;
pub use file::write_failed_thumbnail;
pub use hash::compute_hash;
pub use hash::compute_hash_for_path;
pub use quality::ThumbnailQuality;
pub use sizes::ThumbnailSize;
pub use thumbnailer::Thumbnailer;
