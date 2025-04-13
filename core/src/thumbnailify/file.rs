// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use tracing::{debug, info}; // <-- Logging macros

use image::{DynamicImage, Rgba, RgbaImage};
use png::{Decoder, Encoder};
use url::Url;

use crate::thumbnailify::hash;
use crate::thumbnailify::{error::ThumbnailError, sizes::ThumbnailSize};

pub fn get_thumbnail_path(
    thumbnails_base_dir: &Path,
    host_path: &Path,
    size: ThumbnailSize,
) -> PathBuf {
    let file_uri = get_file_uri(&host_path).unwrap();
    let file_uri_hash = hash::compute_hash(&file_uri);
    get_thumbnail_hash_output(thumbnails_base_dir, &file_uri_hash, size)
}

/// Gets the thumbnail output path using hash and size.
/// Format: `{cache_dir}/thumbnails/{size}/{md5_hash}.png`
pub fn get_thumbnail_hash_output(
    thumbnails_base_dir: &Path,
    hash: &str,
    size: ThumbnailSize,
) -> PathBuf {
    let output_dir = thumbnails_base_dir.join(size.to_string());
    let output_file = format!("{}.png", hash);
    let path = output_dir.join(output_file);

    debug!(
        "Constructed thumbnail hash output path for hash={} size={:?}: {:?}",
        hash, size, path
    );
    path
}

/// Returns the output path for a failed thumbnail marker.
/// This uses the fails folder under the thumbnails cache.
pub fn get_failed_thumbnail_output(thumbnails_base_dir: &Path, hash: &str) -> PathBuf {
    // FIXME don't hardcode app-id.
    let fail_dir = thumbnails_base_dir.join("fail").join("app.fotema.Fotema");
    let output_file = format!("{}.png", hash);
    let path = fail_dir.join(output_file);

    debug!(
        "Constructed fail thumbnail path for hash={}: {:?}",
        hash, path
    );
    path
}

/// Writes a failed thumbnail using an empty (1x1 transparent) DynamicImage.
pub fn write_failed_thumbnail(fail_path: &Path, source_path: &Path) -> Result<(), ThumbnailError> {
    info!(
        "Writing failed thumbnail marker at {:?} for source {:?}",
        fail_path, source_path
    );
    let failed_img: DynamicImage =
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 0])));

    write_out_thumbnail(fail_path, failed_img, source_path)
}

/// Attempts to convert the file path into a file URI.
/// `input` must be a host path.
pub fn get_file_uri(input: &Path) -> Result<String, ThumbnailError> {
    debug!("Attempting to get file URI for path: {:?}", input);
    // Attempt to canonicalize the input to get the full file path.
    let canonical = std::fs::canonicalize(input).unwrap_or_else(|_| {
        debug!(
            "Failed to canonicalize path: {:?}, using the raw path as fallback",
            input
        );
        PathBuf::from(input)
    });
    let url = Url::from_file_path(&canonical).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to convert file path to URL",
        )
    })?;

    debug!("File URI for path {:?} is {}", input, url);
    Ok(url.to_string())
}

/// Writes out the thumbnail as a PNG, embedding:
/// - `Thumb::URI`
/// - `Thumb::Size`
/// - `Thumb::MTime`
pub fn write_out_thumbnail(
    image_path: &Path,
    img: DynamicImage,
    source_image_path: &Path,
) -> Result<(), ThumbnailError> {
    info!(
        "Writing out thumbnail to {:?} from source {:?}",
        image_path, source_image_path
    );

    // FIXME write to temporary file first and then move to final path.
    let file = File::create(image_path)?;

    let rgba_image: RgbaImage = img.to_rgba8();
    let (width, height) = rgba_image.dimensions();
    let buffer = rgba_image.into_raw();

    let mut encoder = Encoder::new(BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);

    let mut writer = encoder.write_header()?;
    writer.write_image_data(&buffer)?;

    debug!("Successfully wrote thumbnail file to {:?}", image_path);
    Ok(())
}

pub fn add_thumbnail_metadata(
    thumb_path: &Path,
    source_image_sandbox_path: &Path,
    source_image_host_path: &Path,
) -> Result<(), ThumbnailError> {
    debug!("Adding thumbnail metadata to {:?}", thumb_path);

    let file_in = File::open(thumb_path)?;
    let reader = BufReader::new(file_in);

    // Decode the PNG
    let decoder = Decoder::new(reader);
    let mut reader = decoder.read_info()?;

    // Extract existing metadata
    let info = reader.info();
    let existing_text = &info.uncompressed_latin1_text.clone();

    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf)?;

    // Re-encode with updated metadata
    // Overwrite the same file (be sure to keep backups in real usage)
    let file_out = File::create(thumb_path)?;
    let w = BufWriter::new(file_out);

    let mut encoder = Encoder::new(w, reader.info().width, reader.info().height);
    encoder.set_color(reader.info().color_type);
    encoder.set_depth(reader.info().bit_depth);

    // Copy existing text chunks into the new file
    for chunk in existing_text {
        encoder.add_text_chunk(chunk.keyword.clone(), chunk.text.clone())?;
    }

    // FIXME hard-coded app-id.
    encoder.add_text_chunk("Software".to_string(), "app.fotema.Fotema".to_string())?;

    let uri = get_file_uri(source_image_host_path)?;
    encoder.add_text_chunk("Thumb::URI".to_string(), uri)?;

    let metadata = std::fs::metadata(source_image_sandbox_path)?;

    let size = metadata.len();
    encoder.add_text_chunk("Thumb::Size".to_string(), size.to_string())?;

    let modified_time = metadata.modified()?;
    let mtime_unix = modified_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    encoder.add_text_chunk("Thumb::MTime".to_string(), mtime_unix.to_string())?;

    // Write out the PNG header
    let mut writer = encoder.write_header()?;

    // Write image data
    writer.write_image_data(&buf)?;

    debug!(
        "Embedded PNG metadata: URI, Size={}, MTime={}",
        size, mtime_unix
    );

    Ok(())
}
