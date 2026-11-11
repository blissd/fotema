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
use tracing::{debug, info, warn}; // <-- Logging macros

use url::Url;

use crate::FlatpakPathBuf;
use crate::thumbnailify::hash;
use crate::thumbnailify::{error::ThumbnailError, sizes::ThumbnailSize};

/// The quality of the JPEG encoder for cached thumbnails. At this quality you
/// cannot see the artefacts at thumbnail size. The file is 5 to 10 times smaller
/// than the equivalent PNG file.
pub const THUMBNAIL_JPEG_QUALITY: u8 = 90;

/// The file extension of the thumbnail format of an older build. The application
/// can still read this format. It replaces the format when it finds it. Refer to
/// `migrate_legacy_png_thumbnails`.
const LEGACY_EXTENSION: &str = "png";

/// The file extension of the thumbnail format of this build.
const EXTENSION: &str = "jpg";

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
    let output_file = format!("{}.{}", hash, EXTENSION);
    let path = output_dir.join(output_file);

    debug!(
        "Constructed thumbnail hash output path for hash={} size={:?}: {:?}",
        hash, size, path
    );
    path
}

/// Find a thumbnail file for the given hash and size. The function also accepts
/// the PNG format of an older build. Thus a new build can use the thumbnail cache
/// of an older build. The function prefers the JPEG format. It uses the `.png`
/// file only when no `.jpg` file exists.
pub fn find_existing_thumbnail(
    thumbnails_base_dir: &Path,
    hash: &str,
    size: ThumbnailSize,
) -> Option<PathBuf> {
    let output_dir = thumbnails_base_dir.join(size.to_string());
    for ext in [EXTENSION, LEGACY_EXTENSION] {
        let candidate = output_dir.join(format!("{}.{}", hash, ext));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Examine if the thumbnail at `thumb_path` agrees with the source image at
/// `host_path`.
///
/// This test does not depend on the file format. The thumbnail is current when
/// its modification time is equal to or later than the modification time of the
/// source. A thumbnail is a JPEG file. Thus the application cannot use the PNG
/// text chunk "Thumb::MTime" of the XDG specification.
pub fn is_thumbnail_up_to_date(thumb_path: &Path, host_path: &Path) -> bool {
    let thumb_mtime = match fs::metadata(thumb_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => {
            debug!("Failed to read thumbnail mtime {:?}: {}", thumb_path, e);
            return false;
        }
    };

    let source_mtime = match fs::metadata(host_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(e) => {
            debug!("Failed to read source mtime {:?}: {}", host_path, e);
            return false;
        }
    };

    thumb_mtime >= source_mtime
}

fn failed_thumbnail_dir(thumbnails_base_dir: &Path) -> PathBuf {
    // FIXME don't hardcode app-id.
    thumbnails_base_dir.join("fail").join("app.fotema.Fotema")
}

pub fn get_failed_thumbnail_output(thumbnails_base_dir: &Path, hash: &str) -> PathBuf {
    let path = failed_thumbnail_dir(thumbnails_base_dir).join(format!("{}.{}", hash, EXTENSION));

    debug!(
        "Constructed fail thumbnail path for hash={}: {:?}",
        hash, path
    );
    path
}

/// Examine if a failure marker exists for the source file.
/// The marker is in the fail folder of the thumbnail cache.
pub fn is_failed(thumbnails_base_dir: &Path, host_path: &Path) -> bool {
    let file_uri = get_file_uri(&host_path).unwrap();
    let file_uri_hash = hash::compute_hash(&file_uri);
    let fail_dir = failed_thumbnail_dir(thumbnails_base_dir);

    // An older build writes the marker as a `.png` file. Accept that file also.
    // Then the application does not try a file again that it cannot process.
    [EXTENSION, LEGACY_EXTENSION]
        .iter()
        .any(|ext| fail_dir.join(format!("{}.{}", file_uri_hash, ext)).exists())
}

/// Write a failure marker for a thumbnail.
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

    // The marker is a sentinel. Only its existence is important. Refer to
    // `is_failed`. Thus a small 1x1 black JPEG file is sufficient. The
    // application writes the marker as JPEG, not as PNG.
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

/// The result of one migration run for the thumbnails of an older build.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MigrationStats {
    /// The number of PNG files that the application encoded as JPEG and deleted.
    pub converted: u64,
    /// The number of PNG files that the application deleted because a JPEG existed.
    pub superseded: u64,
    /// The number of PNG files that the application kept because it cannot read or write them.
    pub failed: u64,
    /// The number of bytes that the migration released. This is the size of the
    /// PNG file minus the size of the new JPEG file.
    pub bytes_freed: i64,
}

impl MigrationStats {
    pub fn total_removed(&self) -> u64 {
        self.converted + self.superseded
    }

    fn merge(&mut self, other: MigrationStats) {
        self.converted += other.converted;
        self.superseded += other.superseded;
        self.failed += other.failed;
        self.bytes_freed += other.bytes_freed;
    }
}

/// Replace each `.png` thumbnail in the cache with a JPEG file. Then delete the
/// PNG file. A PNG thumbnail is 5 to 10 times larger than the equivalent JPEG
/// thumbnail. Thus a cache from an older build uses too much disk space until
/// the application migrates it.
///
/// You can run this function again, and you can stop it at any time. The function
/// converts each thumbnail independently. It writes the JPEG file to a temporary
/// file and then moves that file in one operation. It deletes the PNG file only
/// after the JPEG file is in position. If the function cannot read a PNG file, it
/// keeps that file. Thus no data becomes lost.
pub fn migrate_legacy_png_thumbnails(thumbnails_base_dir: &Path) -> MigrationStats {
    let mut stats = MigrationStats::default();

    let dirs = [
        ThumbnailSize::Small.to_string(),
        ThumbnailSize::Normal.to_string(),
        ThumbnailSize::Large.to_string(),
        ThumbnailSize::XLarge.to_string(),
        ThumbnailSize::XXLarge.to_string(),
    ];

    for dir in dirs {
        stats.merge(migrate_dir(&thumbnails_base_dir.join(dir), false));
    }

    // A failure marker is a 1x1 sentinel. To encode it again gives no result.
    // Thus the function writes a new marker in its place.
    stats.merge(migrate_dir(
        &failed_thumbnail_dir(thumbnails_base_dir),
        true,
    ));

    if stats.total_removed() > 0 || stats.failed > 0 {
        info!(
            "Legacy PNG thumbnail migration: {} converted, {} superseded, {} failed, {} KiB freed",
            stats.converted,
            stats.superseded,
            stats.failed,
            stats.bytes_freed / 1024
        );
    }

    stats
}

fn migrate_dir(dir: &Path, marker_only: bool) -> MigrationStats {
    let mut stats = MigrationStats::default();

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        // If the cache directory does not exist, this is not an error.
        Err(_) => return stats,
    };

    for entry in entries.flatten() {
        let png_path = entry.path();
        if png_path.extension().and_then(|e| e.to_str()) != Some(LEGACY_EXTENSION) {
            continue;
        }

        let png_len = entry.metadata().map(|m| m.len()).unwrap_or(0) as i64;
        let jpg_path = png_path.with_extension(EXTENSION);

        if jpg_path.exists() {
            // A newer build wrote the JPEG file. Thus the PNG file is not necessary.
            match fs::remove_file(&png_path) {
                Ok(()) => {
                    stats.superseded += 1;
                    stats.bytes_freed += png_len;
                }
                Err(e) => {
                    warn!("Failed to delete superseded {:?}: {}", png_path, e);
                    stats.failed += 1;
                }
            }
            continue;
        }

        let written = if marker_only {
            write_marker_jpeg(&jpg_path)
        } else {
            transcode_png_to_jpeg(&png_path, &jpg_path)
        };

        match written {
            Ok(jpg_len) => {
                // Set the modification time of the JPEG file to the modification
                // time of the PNG file. Then the application does not classify the
                // new thumbnail as out of date.
                if let Ok(mtime) = fs::metadata(&png_path).and_then(|m| m.modified())
                    && let Ok(file) = File::options().write(true).open(&jpg_path)
                {
                    let _ = file.set_modified(mtime);
                }

                match fs::remove_file(&png_path) {
                    Ok(()) => {
                        stats.converted += 1;
                        stats.bytes_freed += png_len - jpg_len;
                    }
                    Err(e) => {
                        warn!("Converted but failed to delete {:?}: {}", png_path, e);
                        stats.failed += 1;
                    }
                }
            }
            Err(e) => {
                debug!("Failed to migrate {:?}: {}", png_path, e);
                stats.failed += 1;
            }
        }
    }

    stats
}

/// Encode a PNG thumbnail as a JPEG file. The function returns the size of the
/// new JPEG file.
fn transcode_png_to_jpeg(png_path: &Path, jpg_path: &Path) -> Result<i64, ThumbnailError> {
    let image = image::ImageReader::open(png_path)?
        .with_guessed_format()?
        .decode()?;

    // A JPEG file has no alpha channel. Thus convert the image to RGB.
    let rgb = image.to_rgb8();

    let dir = jpg_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Thumbnail path has no parent directory",
        )
    })?;

    let named_temp = tempfile::Builder::new()
        .prefix("thumb-")
        .suffix(".jpg.tmp")
        .tempfile_in(dir)?;

    {
        let file = BufWriter::new(File::create(named_temp.path())?);
        let mut encoder =
            image::codecs::jpeg::JpegEncoder::new_with_quality(file, THUMBNAIL_JPEG_QUALITY);
        encoder.encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )?;
    }

    let len = named_temp.path().metadata()?.len() as i64;
    named_temp.persist(jpg_path)?;
    Ok(len)
}

/// Write a 1x1 JPEG failure marker in place of a PNG marker of an older build.
fn write_marker_jpeg(jpg_path: &Path) -> Result<i64, ThumbnailError> {
    let file = File::create(jpg_path)?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new(BufWriter::new(file));
    encoder.encode(&[0u8, 0, 0], 1, 1, image::ExtendedColorType::Rgb8)?;
    Ok(jpg_path.metadata()?.len() as i64)
}
