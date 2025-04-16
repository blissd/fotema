// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use image::DynamicImage;
use std::path::{Path, PathBuf};

pub mod error;
pub mod file;
pub mod hash;
pub mod sizes;
pub mod thumbnailer;

pub use error::ThumbnailError;
pub use file::get_thumbnail_path;
pub use file::is_failed;
pub use file::write_failed_thumbnail;
pub use sizes::ThumbnailSize;
pub use thumbnailer::generate_thumbnail;

#[derive(Clone, Debug)]
pub struct Thumbnailer {
    thumbnails_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(thumbnails_path: &Path) -> Thumbnailer {
        Thumbnailer {
            thumbnails_path: thumbnails_path.into(),
        }
    }

    pub fn is_failed(&self, host_path: &Path) -> bool {
        file::is_failed(&self.thumbnails_path, host_path)
    }

    pub fn is_thumbnail_up_to_date(&self, host_path: &Path) -> bool {
        thumbnailer::is_thumbnail_up_to_date(&self.thumbnails_path, host_path)
    }

    pub fn generate_thumbnail(
        &self,
        host_path: &Path,
        sandbox_path: &Path,
        size: ThumbnailSize,
        src_image: DynamicImage,
    ) -> Result<PathBuf, ThumbnailError> {
        thumbnailer::generate_thumbnail(
            &self.thumbnails_path,
            host_path,
            sandbox_path,
            size,
            src_image,
        )
    }
    pub fn write_failed_thumbnail(
        &self,
        host_path: &Path,
        sandbox_path: &Path,
    ) -> Result<(), ThumbnailError> {
        file::write_failed_thumbnail(&self.thumbnails_path, host_path, sandbox_path)
    }
}
