// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;
use fast_image_resize as fr;
use gdk4::prelude::TextureExt;
use glycin;
use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
use image::ExtendedColorType;
use image::ImageEncoder;
use std::io::BufWriter;
use std::io::Write;
use std::num::NonZeroU32;
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
        } else {
            thumbnail_path.parent().map(|p| {
                let _ = std::fs::create_dir_all(p);
            });
        }

        event!(Level::DEBUG, "Standard thumbnail: {:?}", picture_path);
        let thumbnail = Self::fast_thumbnail(picture_path, &thumbnail_path);

        if thumbnail.is_err() {
            event!(Level::DEBUG, "Fallback thumbnail: {:?}", picture_path);
            Self::fallback_thumbnail(picture_path, &thumbnail_path).await?
        }

        Ok(thumbnail_path)
    }

    pub fn fast_thumbnail(path: &Path, thumbnail_path: &Path) -> Result<()> {
        let img = ImageReader::open(path)?.decode()?;

        let width = NonZeroU32::new(img.width()).unwrap();

        let height = NonZeroU32::new(img.height()).unwrap();

        let src_image = fr::Image::from_vec_u8(
            width,
            height,
            img.to_rgba8().into_raw(),
            fr::PixelType::U8x4,
        )?;

        let mut src_view = src_image.view();

        // A square box centred on the image.
        src_view.set_crop_box_to_fit_dst_size(
            NonZeroU32::new(EDGE).expect("Must be EDGE"),
            NonZeroU32::new(EDGE).expect("Must be EDGE"),
            Some((0.5, 0.5)),
        );

        // Multiple RGB channels of source image by alpha channel
        // (not required for the Nearest algorithm)
        //let alpha_mul_div = fr::MulDiv::default();
        //alpha_mul_div
        //    .multiply_alpha_inplace(&mut src_view)
        //    .map_err(|e| PreviewError(format!("fast thumbnail: {}", e)))?;

        // Create container for data of destination image
        let dst_width = NonZeroU32::new(EDGE).expect("Must be EDGE");
        let dst_height = NonZeroU32::new(EDGE).expect("Must be EDGE");
        let mut dst_image = fr::Image::new(dst_width, dst_height, src_image.pixel_type());

        // Get mutable view of destination image data
        let mut dst_view = dst_image.view_mut();

        // Create Resizer instance and resize source image
        // into buffer of destination image
        let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3));
        resizer.resize(&src_view, &mut dst_view)?;

        // Divide RGB channels of destination image by alpha
        // alpha_mul_div
        //     .divide_alpha_inplace(&mut dst_view)?;

        // Write destination image as PNG-file
        // Write to temporary file first and then move so that an interrupted write
        // doesn't result in a corrupt thumbnail

        let temporary_png_file = thumbnail_path.with_extension("tmp");

        let file = std::fs::File::create(&temporary_png_file)?;
        let mut file = BufWriter::new(file);

        PngEncoder::new(&mut file).write_image(
            dst_image.buffer(),
            dst_width.get(),
            dst_height.get(),
            ExtendedColorType::Rgba8,
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
