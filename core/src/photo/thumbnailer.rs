// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;

use image::ImageReader;

use gdk4::prelude::TextureExt;
use glycin;
use std::io::Cursor;
use tracing::error;

use crate::FlatpakPathBuf;
use crate::thumbnailify;

/// Thumbnail operations for photos.
#[derive(Debug, Clone)]
pub struct PhotoThumbnailer {
    thumbnailer: thumbnailify::Thumbnailer,
}

impl PhotoThumbnailer {
    pub fn build(thumbnailer: thumbnailify::Thumbnailer) -> Result<PhotoThumbnailer> {
        Ok(PhotoThumbnailer {
            thumbnailer: thumbnailer,
        })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system and path returned.
    pub async fn thumbnail(&self, path: &FlatpakPathBuf) -> Result<()> {
        if self.thumbnailer.is_failed(&path.host_path) {
            anyhow::bail!("Failed thumbnail marker exists for {:?}", path.host_path);
        }

        self.thumbnail_internal(path).await.map_err(|err| {
            let _ = self.thumbnailer.write_failed_thumbnail(path);
            err
        })
    }

    async fn thumbnail_internal(&self, path: &FlatpakPathBuf) -> Result<()> {
        let file = gio::File::for_path(&path.sandbox_path);
        let loader = glycin::Loader::new(file);
        let image = loader.load().await.map_err(|err| {
            error!("Glycin failed to load file at {:?}", path.sandbox_path);
            err
        })?;

        let frame = image.next_frame().await.map_err(|err| {
            error!(
                "Glycin failed to fetch next frame from {:?}",
                path.sandbox_path
            );
            err
        })?;

        let bytes = frame.texture().save_to_png_bytes();

        let src_image =
            ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png).decode()?;
        /*
                let _ = self.thumbnailer.generate_thumbnail(
                    path,
                    thumbnailify::ThumbnailSize::Large,
                    src_image.clone(),
                )?;
        */
        let _ = self.thumbnailer.generate_all_thumbnails(path, src_image)?;

        Ok(())
    }
}
