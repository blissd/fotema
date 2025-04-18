// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use image::DynamicImage;
use image::ExtendedColorType;
use image::ImageEncoder;
use image::ImageReader;
use image::codecs::png::PngEncoder;

use fast_image_resize as fr;
use fr::images::Image;
use fr::{ResizeOptions, Resizer};
use futures::executor::block_on;

use gdk4::prelude::TextureExt;
use glycin;
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile;
use tracing::debug;

const EDGE: f32 = 512.0;

/// Thumbnail operations for photos.
#[derive(Debug, Clone)]
pub struct Thumbnailer {
    base_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(base_path: &Path) -> Result<Thumbnailer> {
        let base_path = PathBuf::from(base_path).join("photo_thumbnails");
        std::fs::create_dir_all(&base_path)?;

        Ok(Thumbnailer { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system and path returned.
    pub async fn thumbnail(&self, picture_id: &PictureId, picture_path: &Path) -> Result<PathBuf> {
        let thumbnail_path = {
            // Create a directory per 1000 thumbnails
            let partition = (picture_id.id() / 1000) as i32;
            let partition = format!("{:0>4}", partition);
            let file_name = format!("{}_{}x{}.png", picture_id, 200, 200);
            self.base_path.join(partition).join(file_name)
        };

        if let Some(p) = thumbnail_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }

        debug!("Generating thumbnail: {:?}", picture_path);
        Self::sandboxed_thumbnail_async(picture_path, &thumbnail_path).await?;
        Ok(thumbnail_path)
    }

    /// Generate a thumbnail from a file that has already been processed in a Glycin sandbox.
    fn trusted_thumbnail(path: &Path, thumbnail_path: &Path) -> Result<()> {
        let src_image = ImageReader::open(path)?.decode()?.into_rgba8();

        // WARNING src_image, dst_image, and the PngEncoder must all
        // use the _same_ pixel type or the PngEncoder will throw errors
        // about having an unexpected number of bytes.
        // PixelType::U8x3 == RGB8
        // PixelType::U8x4 == RGBA8

        let src_image = DynamicImage::ImageRgba8(src_image);

        let src_width: f32 = src_image.width() as f32;
        let src_height: f32 = src_image.height() as f32;
        let src_longest_edge = if src_width > src_height {
            src_width
        } else {
            src_height
        };

        let scale: f32 = if src_longest_edge <= EDGE {
            1.0
        } else {
            EDGE / (src_longest_edge as f32)
        };

        let dst_width = (src_width * scale) as u32;
        let dst_height = (src_height * scale) as u32;

        let mut dst_image = Image::new(dst_width, dst_height, fr::PixelType::U8x4);

        let mut resizer = Resizer::new();

        resizer.resize(&src_image, &mut dst_image, &ResizeOptions::new())?;

        // Write destination image as PNG-file
        // Write to temporary file first and then move so that an interrupted write
        // doesn't result in a corrupt thumbnail

        let temporary_png_file = thumbnail_path.with_extension("tmp");

        let file = std::fs::File::create(&temporary_png_file)?;
        let mut file = BufWriter::new(file);

        PngEncoder::new(&mut file).write_image(
            dst_image.buffer(),
            dst_width,
            dst_height,
            ExtendedColorType::Rgba8,
        )?;

        file.flush()?;

        std::fs::rename(temporary_png_file, thumbnail_path)?;

        Ok(())
    }

    pub fn sandboxed_thumbnail(source_path: &Path, thumbnail_path: &Path) -> Result<()> {
        block_on(async { Self::sandboxed_thumbnail_async(source_path, thumbnail_path).await })
    }

    /// Copy an image to a PNG file using Glycin, and then use image-rs to compute the thumbnail.
    pub async fn sandboxed_thumbnail_async(
        source_path: &Path,
        thumbnail_path: &Path,
    ) -> Result<()> {
        let file = gio::File::for_path(source_path);

        let loader = glycin::Loader::new(file);

        let image = loader.load().await?;

        let frame = image.next_frame().await?;

        let png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        frame.texture().save_to_png(png_file.path())?;

        Self::trusted_thumbnail(png_file.path(), thumbnail_path)
    }
}
