// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use crate::Error::*;
use crate::Result;
use fast_image_resize as fr;
use gdk4::prelude::TextureExt;
use glycin;
use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
use image::ExtendedColorType;
use image::ImageEncoder;
use std::io::BufWriter;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use tempfile;

const EDGE: u32 = 200;

/// Enrichment operations for photos.
/// Enriches photos with a thumbnail and EXIF metadata.
#[derive(Debug, Clone)]
pub struct Thumbnailer {
    base_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(base_path: &Path) -> Result<Thumbnailer> {
        let base_path = PathBuf::from(base_path).join("photo_thumbnails");
        std::fs::create_dir_all(&base_path)
            .map_err(|e| PreviewError(format!("thumbnail dir: {}", e)))?;

        Ok(Thumbnailer { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system and path returned.
    pub async fn thumbnail(&self, picture_id: &PictureId, picture_path: &Path) -> Result<PathBuf> {
        let thumbnail_path = {
            let file_name = format!("{}_{}x{}.png", picture_id, EDGE, EDGE);
            self.base_path.join(file_name)
        };

        if thumbnail_path.exists() {
            return Ok(thumbnail_path);
        }

        let thumbnail = self.fast_thumbnail(picture_path, &thumbnail_path);

        if thumbnail.is_err() {
            self.fallback_thumbnail(picture_path, &thumbnail_path)
                .await?
        }

        Ok(thumbnail_path)
    }

    fn fast_thumbnail(&self, path: &Path, thumbnail_path: &Path) -> Result<()> {
        let img = ImageReader::open(path)
            .map_err(|e| PreviewError(format!("image open: {}", e)))?
            .decode()
            .map_err(|e| PreviewError(format!("image decode: {}", e)))?;

        let width = NonZeroU32::new(img.width()).unwrap();

        let height = NonZeroU32::new(img.height()).unwrap();

        let src_image = fr::Image::from_vec_u8(
            width,
            height,
            img.to_rgba8().into_raw(),
            fr::PixelType::U8x4,
        )
        .map_err(|e| PreviewError(format!("image save: {}", e)))?;

        let mut src_view = src_image.view();

        // A square box centred on the image.
        src_view.set_crop_box_to_fit_dst_size(
            NonZeroU32::new(EDGE).unwrap(),
            NonZeroU32::new(EDGE).unwrap(),
            Some((0.5, 0.5)),
        );

        // Multiple RGB channels of source image by alpha channel
        // (not required for the Nearest algorithm)
        //let alpha_mul_div = fr::MulDiv::default();
        //alpha_mul_div
        //    .multiply_alpha_inplace(&mut src_view)
        //    .map_err(|e| PreviewError(format!("fast thumbnail: {}", e)))?;

        // Create container for data of destination image
        let dst_width = NonZeroU32::new(EDGE).unwrap();
        let dst_height = NonZeroU32::new(EDGE).unwrap();
        let mut dst_image = fr::Image::new(dst_width, dst_height, src_image.pixel_type());

        // Get mutable view of destination image data
        let mut dst_view = dst_image.view_mut();

        // Create Resizer instance and resize source image
        // into buffer of destination image
        let mut resizer = fr::Resizer::new(fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3));
        resizer
            .resize(&src_view, &mut dst_view)
            .map_err(|e| PreviewError(format!("fast thumbnail resize: {}", e)))?;

        // Divide RGB channels of destination image by alpha
        // alpha_mul_div
        //     .divide_alpha_inplace(&mut dst_view)
        //   .map_err(|e| PreviewError(format!("fast thumbnail: {}", e)))?;

        // Write destination image as PNG-file

        let file = std::fs::File::create(thumbnail_path)
            .map_err(|e| PreviewError(format!("fast thumbnail encode: {}", e)))?;

        let mut file = BufWriter::new(file);

        PngEncoder::new(&mut file)
            .write_image(
                dst_image.buffer(),
                dst_width.get(),
                dst_height.get(),
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| PreviewError(format!("fast thumbnail encode: {}", e)))
    }

    /// Copy an image to a PNG file using Glycin, and then use image-rs to compute the thumbnail.
    /// This is the fallback if image-rs can't decode the original image (such as HEIC images).
    async fn fallback_thumbnail(&self, source_path: &Path, thumbnail_path: &Path) -> Result<()> {
        let file = gio::File::for_path(source_path);

        let image = glycin::Loader::new(file)
            .load()
            .await
            .map_err(|e| PreviewError(format!("Glycin load image: {}", e)))?;

        let frame = image
            .next_frame()
            .await
            .map_err(|e| PreviewError(format!("Glycin image frame: {}", e)))?;

        let png_file = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(|e| PreviewError(format!("Temp file: {}", e)))?;

        frame
            .texture
            .save_to_png(png_file.path())
            .map_err(|e| PreviewError(format!("image save: {}", e)))?;

        self.fast_thumbnail(png_file.path(), thumbnail_path)
    }
}
