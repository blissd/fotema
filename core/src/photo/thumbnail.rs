// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;

use image::ImageReader;

use gdk4::prelude::TextureExt;
use glycin;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::thumbnailify;

/// Thumbnail operations for photos.
#[derive(Debug, Clone)]
pub struct Thumbnailer {
    base_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(thumbnails_base_path: &Path) -> Result<Thumbnailer> {
        Ok(Thumbnailer {
            base_path: thumbnails_base_path.into(),
        })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system and path returned.
    pub async fn thumbnail(&self, host_path: &Path, sandbox_path: &Path) -> Result<PathBuf> {
        if thumbnailify::is_failed(&self.base_path, host_path) {
            return Err(anyhow!("failed thumbnail marker exists"));
        }

        let file = gio::File::for_path(sandbox_path);
        let loader = glycin::Loader::new(file);
        let image = loader.load().await.map_err(|err| {
            let _ = thumbnailify::write_failed_thumbnail(&self.base_path, host_path, sandbox_path);
            err
        })?;

        let frame = image.next_frame().await.map_err(|err| {
            let _ = thumbnailify::write_failed_thumbnail(&self.base_path, host_path, sandbox_path);
            err
        })?;

        let bytes = frame.texture().save_to_png_bytes();

        let src_image = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png)
            .decode()
            .map_err(|err| {
                let _ =
                    thumbnailify::write_failed_thumbnail(&self.base_path, host_path, sandbox_path);
                err
            })?;

        let thumb_path = thumbnailify::generate_thumbnail(
            &self.base_path,
            host_path,
            sandbox_path,
            thumbnailify::ThumbnailSize::XLarge,
            src_image,
        )?;

        Ok(thumb_path)
    }
}
