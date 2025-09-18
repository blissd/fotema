// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Generates high-quality thumbnails for faces selected as the thumbnail for a person.
// Necessary because face thumbnails are extracted from x-large thumbnail images, which
// makes the face thumbnails quite small.

use std::io::Cursor;
use std::path::PathBuf;

use super::model::DetectedFace;
use crate::FlatpakPathBuf;
use crate::thumbnailify;
use crate::thumbnailify::ThumbnailSize;

use anyhow::*;
use gdk4::prelude::TextureExt;
use glycin;
use image::ImageReader;
use tracing::error;

#[derive(Debug, Clone)]
pub struct PersonThumbnailer {
    thumbnailer: thumbnailify::Thumbnailer,
    cache_dir: PathBuf,
}

impl PersonThumbnailer {
    pub fn build(
        thumbnailer: thumbnailify::Thumbnailer,
        cache_dir: impl Into<PathBuf>,
    ) -> PersonThumbnailer {
        let cache_dir: PathBuf = cache_dir.into();
        let large_thumbnail_path = cache_dir.join("face_thumbnails").join("large");
        let _ = std::fs::create_dir_all(large_thumbnail_path);

        PersonThumbnailer {
            thumbnailer,
            cache_dir,
        }
    }

    pub async fn thumbnail(
        &self,
        original_picture: &FlatpakPathBuf,
        face: &DetectedFace,
    ) -> Result<()> {
        let large_thumbnail_path = self.cache_dir.join("face_thumbnails").join("large").join(
            face.small_thumbnail_path
                .file_name()
                .expect("Face thumbnail must have file name"),
        );

        if large_thumbnail_path.exists() {
            return Ok(());
        }

        let file = gio::File::for_path(&original_picture.sandbox_path);
        let loader = glycin::Loader::new(file);

        let original_image = loader.load().await.map_err(|err| {
            error!(
                "Glycin failed to load file at {:?}",
                original_picture.host_path
            );
            err
        })?;

        let frame = original_image.next_frame().await.map_err(|err| {
            error!(
                "Glycin failed to fetch next frame from {:?}",
                original_picture.host_path
            );
            err
        })?;

        // If face was detected in original photo, then the bounds of the detected face
        // map exactly to the source image.
        // If face was detected in the x-large thumbnail image, then the bounds of the detected
        // face map to the thumbnail so must be translated by the scale ration between the
        // source image and the thumbnail.
        let ratio: f32 = if face.is_source_original {
            1.0
        } else {
            let thumb_path = self
                .thumbnailer
                .get_thumbnail_path(&original_picture.host_path, ThumbnailSize::XLarge);
            let file = gio::File::for_path(&thumb_path);
            let loader = glycin::Loader::new(file);
            let thumbnail_image = loader.load().await.map_err(|err| {
                error!("Glycin failed to load file at {:?}", thumb_path);
                err
            })?;

            let original_edge = u32::max(
                original_image.details().height(),
                original_image.details().width(),
            );
            let thumb_edge = u32::max(
                thumbnail_image.details().height(),
                thumbnail_image.details().width(),
            );
            original_edge as f32 / thumb_edge as f32
        };

        let face = face.clone().scale(ratio);

        // FIXME the rest of this code is pretty much a copy and paste from face_extractor.rs

        // Extract face and save to thumbnail.
        // The bounding box is pretty tight, so make it a bit bigger.
        // Also, make the box a square.

        let longest: f32 = f32::max(face.bounds.width, face.bounds.height);
        let mut longest = longest * 1.6;
        let mut half_longest = longest / 2.0;

        let (centre_x, centre_y) = face.centre();

        // Normalize thumbnail to be a square.
        if (original_image.details().width() as f32) < centre_x + half_longest {
            half_longest = original_image.details().width() as f32 - centre_x;
            longest = half_longest * 2.0;
        }
        if (original_image.details().height() as f32) < centre_y + half_longest {
            half_longest = original_image.details().height() as f32 - centre_y;
            longest = half_longest * 2.0;
        }

        if centre_x < half_longest {
            half_longest = centre_x;
            longest = half_longest * 2.0;
        }

        if centre_y < half_longest {
            half_longest = centre_y;
            longest = half_longest * 2.0;
        }

        // Don't panic when x or y would be < zero
        let mut x = centre_x - half_longest;
        if x < 0.0 {
            x = 0.0;
        }
        let mut y = centre_y - half_longest;
        if y < 0.0 {
            y = 0.0;
        }

        let bytes = frame.texture().save_to_png_bytes();

        let original_image = ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png)
            .decode()
            .map_err(|err| {
                error!("Failed to convert to PNG: {:?}", original_picture.host_path);
                err
            })?;

        // FIXME use fast_image_resize instead of image-rs
        let thumbnail = original_image.crop_imm(x as u32, y as u32, longest as u32, longest as u32);
        let thumbnail = thumbnail.thumbnail(256, 256);

        thumbnail.save(&large_thumbnail_path).map_err(|err| {
            error!("Failed to save face thumbnail: {:?}", large_thumbnail_path);
            err
        })?;

        Ok(())
    }
}
