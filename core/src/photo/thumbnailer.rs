// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;

use glycin;

use tracing::error;

use crate::FlatpakPathBuf;
use crate::texture_utils;
use crate::thumbnailify;
use crate::thumbnailify::{ThumbnailQuality, ThumbnailSize};

/// Thumbnail operations for photos.
#[derive(Debug, Clone)]
pub struct PhotoThumbnailer {
    thumbnailer: thumbnailify::Thumbnailer,
}

impl PhotoThumbnailer {
    pub fn build(thumbnailer: thumbnailify::Thumbnailer) -> Result<PhotoThumbnailer> {
        Ok(PhotoThumbnailer { thumbnailer })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system and path returned.
    pub async fn thumbnail(&self, path: &FlatpakPathBuf) -> Result<()> {
        if self.thumbnailer.is_failed(&path.host_path) {
            anyhow::bail!("Failed thumbnail marker exists for {:?}", path.host_path);
        }

        self.thumbnail_internal(path, ThumbnailSize::Large, ThumbnailQuality::High)
            .await
            .map_err(|err| {
                let _ = self.thumbnailer.write_failed_thumbnail(path);
                err
            })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system and path returned.
    pub async fn thumbnail2(
        &self,
        path: &FlatpakPathBuf,
        size: ThumbnailSize,
        quality: ThumbnailQuality,
    ) -> Result<()> {
        if self.thumbnailer.is_failed(&path.host_path) {
            anyhow::bail!("Failed thumbnail marker exists for {:?}", path.host_path);
        }

        self.thumbnail_internal(path, size, quality)
            .await
            .map_err(|err| {
                let _ = self.thumbnailer.write_failed_thumbnail(path);
                err
            })
    }

    async fn thumbnail_internal(
        &self,
        path: &FlatpakPathBuf,
        size: ThumbnailSize,
        quality: ThumbnailQuality,
    ) -> Result<()> {
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

        let src_image = texture_utils::texture_to_rgba(frame.texture())?;

        let _ = self
            .thumbnailer
            .generate_thumbnail(path, size, quality, src_image)?;

        Ok(())
    }
}
