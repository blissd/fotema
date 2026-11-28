// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::io::BufWriter;
use std::{fs, io, path::Path, path::PathBuf};

use tracing::info;

use crate::FlatpakPathBuf;

use super::{
    ThumbnailQuality,
    error::ThumbnailError,
    file,
    file::{
        THUMBNAIL_JPEG_QUALITY, get_failed_thumbnail_output, get_file_uri,
        get_thumbnail_hash_output, is_thumbnail_up_to_date,
    },
    hash::compute_hash,
    sizes::ThumbnailSize,
};

use image::{DynamicImage, ImageBuffer, Rgba};

use fast_image_resize as fr;
use fr::images::Image;
use fr::{ResizeOptions, Resizer};

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

    /// The path of the thumbnail of the given size, when the file exists. The
    /// function also accepts the PNG format of an older build.
    pub fn existing_thumbnail(&self, hash: &str, size: ThumbnailSize) -> Option<PathBuf> {
        file::find_existing_thumbnail(&self.thumbnails_path, hash, size)
    }

    pub fn get_thumbnail_path(&self, host_path: &Path, size: ThumbnailSize) -> PathBuf {
        file::get_thumbnail_path(&self.thumbnails_path, host_path, size)
    }

    /// Find the path of the thumbnail for the given size. If that size does not
    /// exist, find the path of an alternative size. If no thumbnail exists, the
    /// function returns None.
    pub fn nearest_thumbnail(&self, hash: &str, size: ThumbnailSize) -> Option<PathBuf> {
        // Each candidate also accepts the PNG format of an older build. Thus the
        // application uses an old cache and does not classify it as absent.
        if let Some(path) = file::find_existing_thumbnail(&self.thumbnails_path, hash, size) {
            return Some(path);
        }

        use ThumbnailSize::*;
        let fallback_order = match size {
            // TODO Examine if the function must exclude some alternative sizes.
            // A request for a small thumbnail can return an XXLarge thumbnail.
            Small => [Small, Normal, Large, XLarge, XXLarge],
            Normal => [Normal, Large, XLarge, XXLarge, Small],
            Large => [Large, XLarge, XXLarge, Normal, Small],
            XLarge => [XLarge, XXLarge, Large, Normal, Small],
            XXLarge => [XXLarge, XLarge, Large, Normal, Small],
        };

        fallback_order
            .iter()
            .find_map(|s| file::find_existing_thumbnail(&self.thumbnails_path, hash, *s))
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

    /// Convert each PNG thumbnail in this cache to JPEG. Then delete the PNG file.
    pub fn migrate_legacy_png_thumbnails(&self) -> file::MigrationStats {
        file::migrate_legacy_png_thumbnails(&self.thumbnails_path)
    }
}

/// Make a thumbnail for a file that is outside of the Flatpak sandbox.
/// NOTE: the sandbox_path and the host_path can point to a picture or to a video.
/// `thumbnails_base_dir` - the base directory of the thumbnail cache.
/// `host_path` - the path to the file _outside_ the sandbox.
/// `sandbox_path` - the path to the file _inside_ the sandbox.
/// `size` - the standard XDG thumbnail size.
/// `quality` - the thumbnail quality.
/// `src_image` - the image data for the thumbnail. Glycin loads this data in a safe way.
///
/// The function returns the new image. Thus the caller can make smaller sizes
/// from this result.
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
            "A fail marker exists and is up-to-date, refusing to thumbnail {:?}",
            fail_path
        );
        Err(io::Error::other(
            "An up-to-date fail marker exists for this file",
        ))?;
    }

    // Determine the expected output thumbnail path.
    let thumb_path = get_thumbnail_hash_output(thumbnails_base_dir, &hash, size);

    // An older build wrote PNG files. If such a file is current, keep it until
    // the migration task converts it, rather than decoding and encoding it now.
    if let Some(existing) = file::find_existing_thumbnail(thumbnails_base_dir, &hash, size)
        && is_thumbnail_up_to_date(&existing, &path.host_path)
    {
        info!(
            "Cached thumbnail at {:?} is up-to-date, keeping it",
            existing
        );
        return Ok(src_image);
    }

    // Prepare a temporary file in the same directory as the final thumbnail.
    // Using `tempfile_in` ensures that the temp file is on the same filesystem
    // so that we can atomically persist (rename) it.
    let thumb_dir = thumb_path
        .parent()
        .ok_or_else(|| io::Error::other("Thumbnail path has no parent directory"))?;

    fs::create_dir_all(thumb_dir)?;

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

    let thumbnail = fast_image_to_dynamic(&dst_image)?;

    let named_temp = tempfile::Builder::new()
        .prefix("thumb-")
        .suffix(".jpg.tmp")
        .tempfile_in(thumb_dir)?;

    // The application writes a thumbnail as JPEG, not as PNG. For a photo library
    // the JPEG file is 5 to 10 times smaller. At this size you cannot see the loss
    // of quality. A JPEG file has no alpha channel. Thus convert the image to RGB.
    //
    // NOTE: the application cannot write the XDG text chunks "Thumb::URI",
    // "Thumb::MTime" and "Thumb::Size" into a JPEG file. It uses the file
    // modification time instead. Refer to `file::is_thumbnail_up_to_date`.
    let rgb = thumbnail.to_rgb8();

    {
        let file = BufWriter::new(fs::File::create(named_temp.path())?);
        let mut encoder =
            image::codecs::jpeg::JpegEncoder::new_with_quality(file, THUMBNAIL_JPEG_QUALITY);
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )?;
    }

    named_temp.persist(&thumb_path)?;

    // The new JPEG file replaces the thumbnail of an older build at the same hash.
    let legacy_path = thumb_path.with_extension("png");
    if legacy_path.exists() {
        let _ = fs::remove_file(&legacy_path);
    }

    Ok(thumbnail)
}

fn fast_image_to_dynamic(img: &Image) -> Result<DynamicImage, ThumbnailError> {
    let width = img.width();
    let height = img.height();
    let pixels = img.buffer();

    // Make an ImageBuffer<u8, &[u8]> around the slice.
    // Then convert it into an owned buffer.
    let buffer = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(width, height, pixels.to_vec())
        .ok_or(io::Error::other("Failed to create ImageBuffer"))?;

    Ok(DynamicImage::ImageRgba8(buffer))
}
