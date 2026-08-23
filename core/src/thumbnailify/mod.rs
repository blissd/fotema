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
pub use file::find_existing_thumbnail;
pub use file::get_file_uri;
pub use file::get_thumbnail_hash_output;
pub use file::get_thumbnail_path;
pub use file::is_failed;
pub use file::write_failed_thumbnail;
pub use hash::compute_hash;
pub use sizes::ThumbnailSize;
pub use thumbnailer::generate_thumbnail;

use crate::FlatpakPathBuf;

pub fn compute_hash_for_path(host_path: &Path) -> String {
    let file_uri = file::get_file_uri(host_path).unwrap();
    hash::compute_hash(&file_uri)
}

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

    pub fn get_thumbnail_hash_output(&self, hash: &str, size: ThumbnailSize) -> PathBuf {
        get_thumbnail_hash_output(&self.thumbnails_path, hash, size)
    }

    pub fn get_thumbnail_path(&self, host_path: &Path, size: ThumbnailSize) -> PathBuf {
        get_thumbnail_path(&self.thumbnails_path, host_path, size)
    }

    //pub fn nearest_thumbnail_by_dimension(&self, hash: &str, dimension: u32) -> Option<PathBuf> {
    //}

    /**
     * Compute thumbnail path, or sensible fallback if preferred size does not exist.
     * If no thumbnails exist, then return preferred path pointing to absent file.
     */
    pub fn nearest_thumbnail(&self, hash: &str, size: ThumbnailSize) -> Option<PathBuf> {
        // Each candidate tolerates the legacy PNG format so caches from older
        // builds are reused rather than treated as missing.
        if let Some(path) = file::find_existing_thumbnail(&self.thumbnails_path, hash, size) {
            return Some(path);
        }

        use ThumbnailSize::*;
        let fallback_order = match size {
            // TODO figure out if some fallback sizes should be excluded?
            // Do I want a request for a small thumbnail to return an XXLarge?
            Small => [Small, Normal, Large, XLarge, XXLarge],
            Normal => [Normal, Large, XLarge, XXLarge, Small],
            Large => [Large, XLarge, XXLarge, Normal, Small],
            XLarge => [XLarge, XXLarge, Large, Normal, Small],
            XXLarge => [XXLarge, XLarge, Large, Normal, Small],
        };

        fallback_order
            .iter()
            .find_map(|s| file::find_existing_thumbnail(&self.thumbnails_path, hash, *s))
    }

    pub fn generate_thumbnail(
        &self,
        path: &FlatpakPathBuf,
        size: ThumbnailSize,
        src_image: DynamicImage,
    ) -> Result<PathBuf, ThumbnailError> {
        thumbnailer::generate_thumbnail(&self.thumbnails_path, path, size, src_image)
    }

    pub fn generate_all_thumbnails(
        &self,
        path: &FlatpakPathBuf,
        src_image: DynamicImage,
    ) -> Result<(), ThumbnailError> {
        thumbnailer::generate_all_thumbnails(&self.thumbnails_path, path, src_image)
    }

    pub fn write_failed_thumbnail(&self, path: &FlatpakPathBuf) -> Result<(), ThumbnailError> {
        file::write_failed_thumbnail(&self.thumbnails_path, path)
    }
}
