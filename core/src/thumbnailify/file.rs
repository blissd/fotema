// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::{
    fs,
    fs::File,
    io::BufWriter,
    path::{Path, PathBuf},
};
use tracing::{debug, info}; // <-- Logging macros

use url::Url;

use crate::FlatpakPathBuf;
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
/// Format: `{cache_dir}/thumbnails/{size}/{md5_hash}.jpg`
pub fn get_thumbnail_hash_output(
    thumbnails_base_dir: &Path,
    hash: &str,
    size: ThumbnailSize,
) -> PathBuf {
    let output_dir = thumbnails_base_dir.join(size.to_string());
    let output_file = format!("{}.jpg", hash);
    let path = output_dir.join(output_file);

    debug!(
        "Constructed thumbnail hash output path for hash={} size={:?}: {:?}",
        hash, size, path
    );
    path
}

/// Resolve an existing thumbnail file for `hash`/`size`, tolerating the legacy
/// PNG format written by older builds. This lets a newer build adopt an older
/// build's thumbnail cache instead of regenerating it. The current JPEG format
/// is preferred; the `.png` fallback is only used when no `.jpg` exists.
pub fn find_existing_thumbnail(
    thumbnails_base_dir: &Path,
    hash: &str,
    size: ThumbnailSize,
) -> Option<PathBuf> {
    let output_dir = thumbnails_base_dir.join(size.to_string());
    for ext in ["jpg", "png"] {
        let candidate = output_dir.join(format!("{}.{}", hash, ext));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn get_failed_thumbnail_output(thumbnails_base_dir: &Path, hash: &str) -> PathBuf {
    // FIXME don't hardcode app-id.
    let fail_dir = thumbnails_base_dir.join("fail").join("app.fotema.Fotema");
    let output_file = format!("{}.jpg", hash);
    let path = fail_dir.join(output_file);

    debug!(
        "Constructed fail thumbnail path for hash={}: {:?}",
        hash, path
    );
    path
}

/// Returns the output path for a failed thumbnail marker.
/// This uses the fails folder under the thumbnails cache.
pub fn is_failed(thumbnails_base_dir: &Path, host_path: &Path) -> bool {
    let file_uri = get_file_uri(&host_path).unwrap();
    let file_uri_hash = hash::compute_hash(&file_uri);
    let failed_path = get_failed_thumbnail_output(thumbnails_base_dir, &file_uri_hash);
    failed_path.exists()
}

/// Writes a failed thumbnail using an empty (1x1 transparent) DynamicImage.
pub fn write_failed_thumbnail(
    thumbnails_base_dir: &Path,
    path: &FlatpakPathBuf,
) -> Result<(), ThumbnailError> {
    let file_uri = get_file_uri(&path.host_path)?;
    let file_uri_hash = hash::compute_hash(&file_uri);
    let fail_path = get_failed_thumbnail_output(thumbnails_base_dir, &file_uri_hash);

    info!(
        "Writing failed thumbnail marker at {:?} for source {:?}",
        fail_path, path.host_path
    );

    fail_path.parent().as_ref().map(|p| fs::create_dir_all(p));

    // The marker is a sentinel: only its existence matters (see `is_failed`),
    // so a minimal 1x1 black JPEG is enough. Stored as JPEG, not PNG.
    let file = File::create(&fail_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new(BufWriter::new(file));
    encoder.encode(&[0u8, 0, 0], 1, 1, image::ExtendedColorType::Rgb8)?;

    debug!("Successfully wrote failure marker file to {:?}", fail_path);
    Ok(())
}

/// Attempts to convert the file path into a file URI.
/// `input` must be a host path.
pub fn get_file_uri(input: &Path) -> Result<String, ThumbnailError> {
    debug!("Attempting to get file URI for path: {:?}", input);
    // Attempt to canonicalize the input to get the full file path.#
    // `canonicalize()` will fail if `host_path` does not exist... which means
    // that it will __never work__ inside the Flatpak sandbox.

    //let canonical = std::fs::canonicalize(input).unwrap_or_else(|_| {
    //    debug!(
    //        "Failed to canonicalize path: {:?}, using the raw path as fallback",
    //        input
    //    );
    //    PathBuf::from(input)
    //});
    let canonical = PathBuf::from(input);

    let url = Url::from_file_path(&canonical).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Failed to convert file path to URL",
        )
    })?;

    debug!("File URI for path {:?} is {}", input, url);
    Ok(url.to_string())
}
