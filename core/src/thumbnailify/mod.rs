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
        let preferred = file::get_thumbnail_hash_output(&self.thumbnails_path, hash, size);

        if preferred.exists() {
            Some(preferred)
        } else {
            let xxlarge = file::get_thumbnail_hash_output(
                &self.thumbnails_path,
                hash,
                ThumbnailSize::XXLarge,
            );
            let xlarge =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::XLarge);
            let large =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::Large);
            let normal =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::Normal);
            let small =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::Small);

            let paths = match size {
                // TODO figure out if some fallback sizes should be excluded?
                // Do I want a request for a small thumbnail to return an XXLarge?
                ThumbnailSize::Small => [small, normal, large, xlarge, xxlarge],
                ThumbnailSize::Normal => [normal, large, xlarge, xxlarge, small],
                ThumbnailSize::Large => [large, xlarge, xxlarge, normal, small],
                ThumbnailSize::XLarge => [xlarge, xxlarge, large, normal, small],
                ThumbnailSize::XXLarge => [xxlarge, xlarge, large, normal, small],
            };

            paths.iter().find(|path| path.exists()).cloned()
        }
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
