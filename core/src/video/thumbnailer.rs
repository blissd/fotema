// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::thumbnailify;
use anyhow::*;
use image::ImageReader;
use std::path::Path;
use std::process::Command;
use std::result::Result::Ok;
use tempfile;

/// Thumbnail operations for videos.
#[derive(Debug, Clone)]
pub struct VideoThumbnailer {
    thumbnailer: thumbnailify::Thumbnailer,
}

impl VideoThumbnailer {
    pub fn build(thumbnailer: thumbnailify::Thumbnailer) -> Result<VideoThumbnailer> {
        Ok(VideoThumbnailer {
            thumbnailer: thumbnailer,
        })
    }

    /// Computes a preview for a video
    pub fn thumbnail(&self, host_path: &Path, sandbox_path: &Path) -> Result<()> {
        if self.thumbnailer.is_failed(host_path) {
            anyhow::bail!(
                "Failed thumbnail marker exists for {:?}",
                host_path.to_string_lossy()
            );
        }

        // Extract first frame of video for thumbnail
        let temporary_png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        // ffmpeg command will extract the first frame and save it as a PNG file.
        let status = Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("error")
            .arg("-y") // temp file will already exist, so allow overwriting
            .arg("-i")
            .arg(sandbox_path.as_os_str())
            .arg("-update")
            .arg("true")
            .arg("-vf")
            .arg(r"select=eq(n\,0)") // select frame zero
            .arg(temporary_png_file.path())
            .status()?;

        if !status.success() {
            let _ = self
                .thumbnailer
                .write_failed_thumbnail(&host_path, sandbox_path);

            anyhow::bail!("FFMpeg exited with status {:?}", status.code());
        }

        let src_image = ImageReader::open(&temporary_png_file)?
            .decode()
            .map_err(|err| {
                let _ = self
                    .thumbnailer
                    .write_failed_thumbnail(&host_path, sandbox_path);
                err
            })?;

        let _ = self.thumbnailer.generate_thumbnail(
            host_path,
            sandbox_path,
            thumbnailify::ThumbnailSize::Large,
            src_image,
        )?;

        Ok(())
    }
}
