// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;

use gdk4::prelude::TextureExt;
use glycin;
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
        Ok(PhotoThumbnailer { thumbnailer })
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

        // Download raw RGBA pixels straight from the decoded texture instead of
        // round-tripping through a full-resolution PNG encode + decode, which
        // dominated photo thumbnailing (the encode alone cost more than the
        // actual image decode).
        let texture = frame.texture();
        let width = texture.width() as u32;
        let height = texture.height() as u32;

        let mut downloader = gdk4::TextureDownloader::new(&texture);
        downloader.set_format(gdk4::MemoryFormat::R8g8b8a8);
        let (bytes, stride) = downloader.download_bytes();

        let row_bytes = width as usize * 4;
        let data = if stride == row_bytes {
            bytes.to_vec()
        } else {
            // Strip row padding so the buffer is tightly packed for `image`.
            let mut packed = Vec::with_capacity(row_bytes * height as usize);
            for y in 0..height as usize {
                let start = y * stride;
                packed.extend_from_slice(&bytes[start..start + row_bytes]);
            }
            packed
        };

        let buffer = image::RgbaImage::from_raw(width, height, data)
            .ok_or_else(|| anyhow!("Texture buffer size mismatch for {:?}", path.sandbox_path))?;
        let src_image = image::DynamicImage::ImageRgba8(buffer);

        let _ = self.thumbnailer.generate_all_thumbnails(path, src_image)?;

        Ok(())
    }
}
