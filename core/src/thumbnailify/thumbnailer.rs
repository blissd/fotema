// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::BufWriter;
use std::{fs, io, path::Path, path::PathBuf, time::UNIX_EPOCH};

use tracing::info;

use crate::FlatpakPathBuf;

use super::{
    ThumbnailQuality,
    error::ThumbnailError,
    file,
    file::{
        get_failed_thumbnail_output, get_file_uri, get_thumbnail_hash_output,
        is_thumbnail_up_to_date,
    },
    hash::compute_hash,
    sizes::ThumbnailSize,
};

use image::{DynamicImage, ImageBuffer, Rgba};

use fast_image_resize as fr;
use fr::images::Image;
use fr::{ResizeOptions, Resizer};

use png::Encoder as ExtendedPngEncoder;

use tempfile;

#[derive(Clone, Debug)]
pub struct Thumbnailer {
    thumbnails_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(thumbnails_path: &Path) -> Thumbnailer {
        Thumbnailer {
            thumbnails_path: thumbnails_path.into(),
        }
    }

    pub fn is_failed(&self, host_path: &Path) -> bool {
        file::is_failed(&self.thumbnails_path, host_path)
    }

    pub fn is_thumbnail_up_to_date(&self, host_path: &Path) -> bool {
        file::is_thumbnail_up_to_date(&self.thumbnails_path, host_path)
    }

    pub fn get_thumbnail_hash_output(&self, hash: &str, size: ThumbnailSize) -> PathBuf {
        file::get_thumbnail_hash_output(&self.thumbnails_path, hash, size)
    }

    pub fn get_thumbnail_path(&self, host_path: &Path, size: ThumbnailSize) -> PathBuf {
        file::get_thumbnail_path(&self.thumbnails_path, host_path, size)
    }

    /**
     * Compute thumbnail path, or sensible fallback if preferred size does not exist.
     * If no thumbnails exist, then return preferred path pointing to absent file.
     */
    pub fn nearest_thumbnail(&self, hash: &str, size: ThumbnailSize) -> Option<PathBuf> {
        let preferred = file::get_thumbnail_hash_output(&self.thumbnails_path, hash, size);

        if preferred.exists() {
            Some(preferred)
        } else {
            let xxlarge = file::get_thumbnail_hash_output(
                &self.thumbnails_path,
                hash,
                ThumbnailSize::XXLarge,
            );
            let xlarge =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::XLarge);
            let large =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::Large);
            let normal =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::Normal);
            let small =
                file::get_thumbnail_hash_output(&self.thumbnails_path, hash, ThumbnailSize::Small);

            let paths = match size {
                // TODO figure out if some fallback sizes should be excluded?
                // Do I want a request for a small thumbnail to return an XXLarge?
                ThumbnailSize::Small => [small, normal, large, xlarge, xxlarge],
                ThumbnailSize::Normal => [normal, large, xlarge, xxlarge, small],
                ThumbnailSize::Large => [large, xlarge, xxlarge, normal, small],
                ThumbnailSize::XLarge => [xlarge, xxlarge, large, normal, small],
                ThumbnailSize::XXLarge => [xxlarge, xlarge, large, normal, small],
            };

            paths.iter().find(|path| path.exists()).cloned()
        }
    }

    pub fn generate_thumbnail(
        &self,
        path: &FlatpakPathBuf,
        size: ThumbnailSize,
        quality: ThumbnailQuality,
        src_image: DynamicImage,
    ) -> Result<(), ThumbnailError> {
        generate_thumbnail(&self.thumbnails_path, path, size, quality, src_image)?;
        Ok(())
    }

    pub fn write_failed_thumbnail(&self, path: &FlatpakPathBuf) -> Result<(), ThumbnailError> {
        file::write_failed_thumbnail(&self.thumbnails_path, path)
    }
}

/// Generate a thumbnail for a file that exists outside of the Flatpak sandbox.
/// NOTE: the sandbox_path/host_path could point to a picture or a video.
/// `thumbnails_base_dir` - thumbnail base directory
/// `host_path` - path _outside_ sandbox to file we are generating thumbnail for.
/// `sandbox_path` - path _inside_ sandbox to file we are generating thumbnail for.
/// `size` - standard XDG thumbnail size.
/// `quality` - thumbnail quality.
/// `src_image` - image data for thumbnail. Image data will have been loaded in a safe way using Glycin.
pub fn generate_thumbnail(
    thumbnails_base_dir: &Path,
    path: &FlatpakPathBuf,
    size: ThumbnailSize,
    quality: ThumbnailQuality,
    src_image: DynamicImage,
) -> Result<DynamicImage, ThumbnailError> {
    // info!("Generating thumbnail for hostpath: {:?}", host_path);

    // `canonicalize()` will fail if `host_path` does not exist... which means
    // that it will __never work__ inside the Flatpak sandbox.
    // let abs_path = host_path.canonicalize()?;

    //let _ = abs_path
    //    .to_str()
    //   .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid file path"))?;

    let file_uri = get_file_uri(&path.host_path)?;

    // Compute the MD5 hash from the file URI.
    let hash = compute_hash(&file_uri);

    // Check if the fail marker exists and is up to date
    let fail_path = get_failed_thumbnail_output(thumbnails_base_dir, &hash);
    if fail_path.exists() && is_thumbnail_up_to_date(&fail_path, &path.sandbox_path) {
        info!(
            "A fail marker exists and is up-to-date, returning fail marker at {:?}",
            fail_path
        );
        Err(io::Error::new(
            io::ErrorKind::Other,
            "Thumbnail path has no parent directory",
        ))?;
    }

    // Determine the expected output thumbnail path.
    let thumb_path = get_thumbnail_hash_output(thumbnails_base_dir, &hash, size);

    // If the thumbnail already exists and is up to date, return it immediately.
    if thumb_path.exists() && is_thumbnail_up_to_date(&thumb_path, &path.host_path) {
        info!(
            "Cached thumbnail at {:?} is up-to-date, returning it",
            thumb_path
        );
        // FIXME load and return existing thumbnail image
        return Ok(src_image);
    }
    // Prepare a temporary file in the same directory as the final thumbnail.
    // Using `tempfile_in` ensures that the temp file is on the same filesystem
    // so that we can atomically persist (rename) it.
    let thumb_dir = thumb_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            "Thumbnail path has no parent directory",
        )
    })?;

    fs::create_dir_all(thumb_dir)?;

    let named_temp = tempfile::Builder::new()
        .prefix("thumb-")
        .suffix(".png.tmp")
        .tempfile_in(thumb_dir)?;

    let temp_path = named_temp.path().to_owned();

    let dimension = size.to_dimension() as f32;

    let src_image = DynamicImage::ImageRgba8(src_image.into());

    let src_width: f32 = src_image.width() as f32;
    let src_height: f32 = src_image.height() as f32;
    let src_longest_edge = f32::max(src_width, src_height);

    let scale: f32 = f32::min(1.0, dimension / src_longest_edge);

    let dst_width = (src_width * scale) as u32;
    let dst_height = (src_height * scale) as u32;

    let mut dst_image = Image::new(dst_width, dst_height, fr::PixelType::U8x4);

    let filter_type = match quality {
        ThumbnailQuality::Normal => fast_image_resize::FilterType::Hamming,
        ThumbnailQuality::High => fast_image_resize::FilterType::Lanczos3,
    };

    let mut resizer = Resizer::new();
    let resize_options =
        ResizeOptions::new().resize_alg(fast_image_resize::ResizeAlg::Convolution(filter_type));

    resizer.resize(&src_image, &mut dst_image, &resize_options)?;

    let file = std::fs::File::create(&temp_path)?;
    let file = BufWriter::new(file);

    let mut encoder = ExtendedPngEncoder::new(file, dst_width, dst_height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    // FIXME hard-coded app-id
    encoder.add_text_chunk("Software".to_string(), "app.fotema.Fotema".to_string())?;

    let uri = get_file_uri(&path.host_path)?;
    encoder.add_text_chunk("Thumb::URI".to_string(), uri)?;

    let metadata = std::fs::metadata(&path.sandbox_path)?;

    let size = metadata.len();
    encoder.add_text_chunk("Thumb::Size".to_string(), size.to_string())?;

    let modified_time = metadata.modified()?;
    let mtime_unix = modified_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    encoder.add_text_chunk("Thumb::MTime".to_string(), mtime_unix.to_string())?;

    // TODO image width/height, video duration.
    // See https://specifications.freedesktop.org/thumbnail-spec/latest/creation.html

    // Write out the PNG header
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&dst_image.buffer())?;
    drop(writer); // flush

    named_temp.persist(&thumb_path)?;

    fast_image_to_dynamic(&dst_image)
}

fn fast_image_to_dynamic(img: &Image) -> Result<DynamicImage, ThumbnailError> {
    let width = img.width();
    let height = img.height();
    let pixels = img.buffer();

    // Create ImageBuffer<u8, &[u8]> by wrapping the slice
    // Then convert to owned buffer
    let buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, pixels.to_vec()).ok_or(
        io::Error::new(io::ErrorKind::Other, "Failed to create ImageBuffer"),
    )?;

    Ok(DynamicImage::ImageRgba8(buffer))
}
