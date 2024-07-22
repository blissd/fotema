// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use image::codecs::png::PngEncoder;
use image::DynamicImage;
use image::ExtendedColorType;
use image::ImageEncoder;
use image::ImageReader;

use fast_image_resize as fr;
use fr::images::Image;
use fr::{ResizeOptions, Resizer};

use gdk4::prelude::TextureExt;
use glycin;
use std::io::BufWriter;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{event, Level};

use tempfile;

const EDGE: u32 = 200;

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
            let file_name = format!("{}_{}x{}.png", picture_id, EDGE, EDGE);
            self.base_path.join(partition).join(file_name)
        };

        if thumbnail_path.exists() {
            return Ok(thumbnail_path);
        } else if let Some(p) = thumbnail_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }

        event!(Level::DEBUG, "Standard thumbnail: {:?}", picture_path);
        let thumbnail = Self::fast_thumbnail(picture_path, &thumbnail_path);

        if thumbnail.is_err() {
            event!(Level::DEBUG, "Fallback thumbnail: {:?}", picture_path);
            Self::fallback_thumbnail(picture_path, &thumbnail_path).await?;
        }

        Ok(thumbnail_path)
    }

    pub fn fast_thumbnail(path: &Path, thumbnail_path: &Path) -> Result<()> {
        let src_image = ImageReader::open(path)?.decode()?.into_rgb8();

        // WARNING src_image, dst_image, and the PngEncoder must all
        // use the _same_ pixel type or the PngEncoder will throw errors
        // about having an unexpected number of bytes.
        // PixelType::U8x3 == RGB8
        // PixelType::U8x4 == RGBA8
        //
        // For now I'm using RGB, not RGBA, because I don't think an alpha channel
        // makes sense for thumbnails.

        let src_image = DynamicImage::ImageRgb8(src_image);

        let mut dst_image = Image::new(EDGE, EDGE, fr::PixelType::U8x3);

        let mut resizer = Resizer::new();

        resizer.resize(
            &src_image,
            &mut dst_image,
            &ResizeOptions::new().fit_into_destination(Some((0.5, 0.5))),
        )?;

        // Write destination image as PNG-file
        // Write to temporary file first and then move so that an interrupted write
        // doesn't result in a corrupt thumbnail

        let temporary_png_file = thumbnail_path.with_extension("tmp");

        let file = std::fs::File::create(&temporary_png_file)?;
        let mut file = BufWriter::new(file);

        PngEncoder::new(&mut file).write_image(
            dst_image.buffer(),
            EDGE,
            EDGE,
            ExtendedColorType::Rgb8,
        )?;

        file.flush()?;

        std::fs::rename(temporary_png_file, thumbnail_path)?;

        Ok(())
    }

    /// Copy an image to a PNG file using Glycin, and then use image-rs to compute the thumbnail.
    /// This is the fallback if image-rs can't decode the original image (such as HEIC images).
    pub async fn fallback_thumbnail(source_path: &Path, thumbnail_path: &Path) -> Result<()> {
        let file = gio::File::for_path(source_path);

        let image = glycin::Loader::new(file).load().await?;

        let frame = image.next_frame().await?;

        let png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        frame.texture.save_to_png(png_file.path())?;

        Self::fast_thumbnail(png_file.path(), thumbnail_path)
    }
}
