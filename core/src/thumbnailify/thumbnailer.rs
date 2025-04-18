// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    fs,
    fs::File,
    io,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tracing::{debug, info};

use crate::thumbnailify::{
    error::ThumbnailError,
    file::{get_failed_thumbnail_output, get_file_uri, get_thumbnail_hash_output},
    hash::compute_hash,
    sizes::ThumbnailSize,
};
use png::Decoder;

use image::DynamicImage;

use fast_image_resize as fr;
use fr::images::Image;
use fr::{ResizeOptions, Resizer};

use png::Encoder as ExtendedPngEncoder;
use std::io::BufWriter;

use tempfile;

/// Checks whether the thumbnail file at `thumb_path` is up to date with respect
/// to the source image at `source_path`. It verifies two metadata fields in the PNG:
///
/// - "Thumb::MTime": the source file's modification time (in seconds since UNIX_EPOCH)
/// - "Thumb::Size": the source file's size in bytes (only checked if present)
///
/// Returns true if "Thumb::MTime" is present and matches the source file's modification time,
/// and if "Thumb::Size" is present it must match the source file's size.
pub fn is_thumbnail_up_to_date(thumb_path: &Path, source_path: &Path) -> bool {
    debug!(
        "Checking if thumbnail at {:?} is up-to-date with source {:?}",
        thumb_path, source_path
    );

    let file = match File::open(thumb_path) {
        Ok(f) => f,
        Err(e) => {
            debug!("Failed to open thumbnail {:?}: {}", thumb_path, e);
            return false;
        }
    };

    let decoder = Decoder::new(file);
    let reader = match decoder.read_info() {
        Ok(r) => r,
        Err(e) => {
            debug!("Failed to read PNG info for {:?}: {}", thumb_path, e);
            return false;
        }
    };

    let texts = &reader.info().uncompressed_latin1_text;

    let thumb_mtime_str = match texts.iter().find(|c| c.keyword == "Thumb::MTime") {
        Some(c) => &c.text,
        None => {
            debug!("Thumbnail missing 'Thumb::MTime' metadata chunk.");
            return false;
        }
    };
    let thumb_mtime = thumb_mtime_str.parse::<u64>().unwrap_or(0);

    let source_metadata = match std::fs::metadata(source_path) {
        Ok(m) => m,
        Err(e) => {
            debug!("Failed to get metadata of source {:?}: {}", source_path, e);
            return false;
        }
    };

    let source_modified_time = match source_metadata.modified() {
        Ok(mt) => mt.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        Err(e) => {
            debug!(
                "Failed to read modified time of source {:?}: {}",
                source_path, e
            );
            return false;
        }
    };

    if thumb_mtime != source_modified_time {
        debug!(
            "Thumb::MTime mismatch: thumbnail={} source={}",
            thumb_mtime, source_modified_time
        );
        return false;
    }

    if let Some(chunk) = texts.iter().find(|c| c.keyword == "Thumb::Size") {
        let thumb_size = chunk.text.parse::<u64>().unwrap_or(0);
        let source_file_size = source_metadata.len();
        if thumb_size != source_file_size {
            debug!(
                "Thumb::Size mismatch: thumbnail={} source={}",
                thumb_size, source_file_size
            );
            return false;
        }
    }

    debug!(
        "Thumbnail at {:?} is up-to-date with source {:?}",
        thumb_path, source_path
    );
    true
}

/// Generate a thumbnail for a file that exists outside of the Flatpak sandbox.
/// NOTE: the sandbox_path/host_path could point to a picture or a video.
/// `thumbnails_base_dir` - thumbnail base directory
/// `host_path` - path _outside_ sandbox to file we are generating thumbnail for.
/// `sandbox_path` - path _inside_ sandbox to file we are generating thumbnail for.
/// `size` - standard XDG thumbnail size.
/// `src_image` - image data for thumbnail. Image data will have been loaded in a safe way using Glycin.
pub fn generate_thumbnail(
    thumbnails_base_dir: &Path,
    host_path: &Path,
    sandbox_path: &Path,
    size: ThumbnailSize,
    src_image: DynamicImage,
) -> Result<PathBuf, ThumbnailError> {
    let abs_path = host_path.canonicalize()?;

    let _ = abs_path
        .to_str()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid file path"))?;

    let file_uri = get_file_uri(host_path)?;

    // Compute the MD5 hash from the file URI.
    let hash = compute_hash(&file_uri);

    // Check if the fail marker exists and is up to date
    let fail_path = get_failed_thumbnail_output(thumbnails_base_dir, &hash);
    if fail_path.exists() && is_thumbnail_up_to_date(&fail_path, sandbox_path) {
        info!(
            "A fail marker exists and is up-to-date, returning fail marker at {:?}",
            fail_path
        );
        return Ok(fail_path);
    }

    // Determine the expected output thumbnail path.
    let thumb_path = get_thumbnail_hash_output(thumbnails_base_dir, &hash, size);

    // If the thumbnail already exists and is up to date, return it immediately.
    if thumb_path.exists() && is_thumbnail_up_to_date(&thumb_path, sandbox_path) {
        info!(
            "Cached thumbnail at {:?} is up-to-date, returning it",
            thumb_path
        );
        return Ok(thumb_path);
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
    let src_longest_edge = if src_width > src_height {
        src_width
    } else {
        src_height
    };

    let scale: f32 = if src_longest_edge <= dimension {
        1.0
    } else {
        dimension / (src_longest_edge as f32)
    };

    let dst_width = (src_width * scale) as u32;
    let dst_height = (src_height * scale) as u32;
    let mut dst_image = Image::new(dst_width, dst_height, fr::PixelType::U8x4);

    let mut resizer = Resizer::new();
    resizer.resize(&src_image, &mut dst_image, &ResizeOptions::new())?;

    let file = std::fs::File::create(&temp_path)?;
    let file = BufWriter::new(file);

    let mut encoder = ExtendedPngEncoder::new(file, dst_width, dst_height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    // FIXME hard-coded app-id
    encoder.add_text_chunk("Software".to_string(), "app.fotema.Fotema".to_string())?;

    let uri = get_file_uri(&host_path)?;
    encoder.add_text_chunk("Thumb::URI".to_string(), uri)?;

    let metadata = std::fs::metadata(&sandbox_path)?;

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

    return Ok(thumb_path.into());
}
