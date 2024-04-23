// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use crate::photo::model::{PhotoExtra, PictureId};
use crate::Error::*;
use crate::Result;
use fast_image_resize as fr;
use gdk4::prelude::TextureExt;
use glycin;
use image::codecs::png;
use image::codecs::png::PngEncoder;
use image::io::Reader as ImageReader;
use image::ColorType;
use image::DynamicImage;
use image::ExtendedColorType;
use image::ImageEncoder;
use std::io::prelude::*;
use std::io::BufWriter;
use std::io::Cursor;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};

use tempfile;

const EDGE: u32 = 200;

/// Enrichment operations for photos.
/// Enriches photos with a thumbnail and EXIF metadata.
#[derive(Debug, Clone)]
pub struct Enricher {
    base_path: PathBuf,
}

impl Enricher {
    pub fn build(base_path: &Path) -> Result<Enricher> {
        let base_path = PathBuf::from(base_path).join("photo_thumbnails");
        let _ = std::fs::create_dir_all(&base_path);
        Ok(Enricher { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub async fn enrich(&self, picture_id: &PictureId, picture_path: &Path) -> Result<PhotoExtra> {
        let mut extra = PhotoExtra::default();

        let thumbnail_path = {
            let file_name = format!("{}_{}x{}.png", picture_id, EDGE, EDGE);
            self.base_path.join(file_name)
        };

        let result = self
            .compute_thumbnail(picture_path, &thumbnail_path)
            .await
            .map_err(|e| PreviewError(format!("save photo thumbnail: {}", e)));

        if result.is_ok() {
            extra.thumbnail_path = Some(thumbnail_path.clone());
        } else {
            println!("Picture thumbnail error: {:?}", result);
        }

        if let Ok(metadata) = Metadata::from_path(picture_path) {
            extra.exif_created_at = metadata.created_at;
            extra.exif_modified_at = metadata.modified_at;
            extra.exif_lens_model = metadata.lens_model;
            extra.content_id = metadata.content_id;
        }

        Ok(extra)
    }
    async fn compute_thumbnail(&self, picture_path: &Path, thumbnail_path: &Path) -> Result<()> {
        if thumbnail_path.exists() {
            return Ok(());
        }

        let thumbnail = self.fast_thumbnail(picture_path);
        //.or_else(|_| self.standard_thumbnail(picture_path))

        let thumbnail = if thumbnail.is_err() {
            self.fallback_thumbnail(picture_path).await
        } else {
            thumbnail
        }?;

        thumbnail
            .save(thumbnail_path)
            .or_else(|e| {
                // let _ = std::fs::remove_file(&thumbnail_path);
                Err(e) // don't lose original error
            })
            .map_err(|e| PreviewError(format!("image save: {}", e)))?;

        Ok(())
    }

    fn fast_thumbnail(&self, path: &Path) -> Result<DynamicImage> {
        let img = ImageReader::open(path)
            .map_err(|e| PreviewError(format!("image open: {}", e)))?
            .decode()
            .map_err(|e| PreviewError(format!("image decode: {}", e)))?;

        let width = NonZeroU32::new(img.width()).unwrap();

        let height = NonZeroU32::new(img.height()).unwrap();

        let mut src_image = fr::Image::from_vec_u8(
            width,
            height,
            img.to_rgba8().into_raw(),
            fr::PixelType::U8x4,
        )
        .map_err(|e| PreviewError(format!("image save: {}", e)))?;

        // A square box centred on the image.
        /*
        let crop_box = if img.width() == img.height() {
            fr::CropBox{left: 0.0, top: 0.0, width: img.width() as f64, height: img.height() as f64}
        } else if img.width() < img.height() {
            let h = (img.height() - img.width()) as f64 / 2.0;
            fr::CropBox{left: 0.0, top: h, width: img.width() as f64, height: img.width() as f64}
        } else {
            let w = (img.width() - img.height()) as f64 / 2.0;
            fr::CropBox{left: w, top: 0.0, width: img.height() as f64, height: img.height() as f64}
        };


        let mut src_view = src_image.view();
        src_view.set_crop_box(crop_box)
            .map_err(|e| PreviewError(format!("fast thumbnail crop: {}", e)))?;
            */

        let mut src_view = src_image.view();
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
        let mut result_buf = BufWriter::new(Vec::new());
        PngEncoder::new(&mut result_buf)
            .write_image(
                dst_image.buffer(),
                dst_width.get(),
                dst_height.get(),
                ExtendedColorType::Rgba8,
            )
            .map_err(|e| PreviewError(format!("fast thumbnail encode: {}", e)))?;

        let result_buf = result_buf
            .into_inner()
            .map_err(|e| PreviewError(format!("fast thumbnail buf: {}", e)))?;

        image::load_from_memory_with_format(&result_buf, image::ImageFormat::Png)
            .map_err(|e| PreviewError(format!("fast thumbnail memload: {}", e)))
    }

    fn standard_thumbnail(&self, path: &Path) -> Result<DynamicImage> {
        let img = ImageReader::open(path)
            .map_err(|e| PreviewError(format!("image open: {}", e)))?
            .decode()
            .map_err(|e| PreviewError(format!("image decode: {}", e)))?;

        let img = if img.width() == img.height() && img.width() == EDGE {
            return Ok(img);
        } else if img.width() == img.height() {
            img
        } else if img.width() < img.height() {
            let h = (img.height() - img.width()) / 2;
            img.crop_imm(0, h, img.width(), img.width())
        } else {
            let w = (img.width() - img.height()) / 2;
            img.crop_imm(w, 0, img.height(), img.height())
        };

        let img = img.thumbnail(EDGE, EDGE);
        Ok(img)
    }

    /// Copy an image to a PNG file using Glycin, and then use image-rs to compute the thumbnail.
    /// This is the fallback if image-rs can't decode the original image (such as HEIC images).
    async fn fallback_thumbnail(&self, path: &Path) -> Result<DynamicImage> {
        let file = gio::File::for_path(path);

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

        self.fast_thumbnail(png_file.path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn picture_dir() -> PathBuf {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        test_data_dir
    }

    #[test]
    fn test_to_square() {
        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Frog.jpg");

        let target_dir = PathBuf::from("target");
        let prev = Previewer::build(&target_dir).unwrap();
        let img = prev.from_path(&test_file).unwrap();
        let output = target_dir.join("out.jpg");
        let _ = img.save(&output);
    }
}
