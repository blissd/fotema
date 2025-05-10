// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::FlatpakPathBuf;
use crate::thumbnailify;

use anyhow::*;
use image::ImageReader;
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
    pub fn thumbnail(&self, path: &FlatpakPathBuf) -> Result<()> {
        if self.thumbnailer.is_failed(&path.host_path) {
            anyhow::bail!("Failed thumbnail marker exists for {:?}", path.host_path);
        }

        self.thumbnail_internal(path).map_err(|err| {
            let _ = self.thumbnailer.write_failed_thumbnail(path);
            err
        })
    }

    pub fn thumbnail_internal(&self, path: &FlatpakPathBuf) -> Result<()> {
        // Extract first frame of video for thumbnail
        let temporary_png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        // ffmpeg command will extract the first frame and save it as a PNG file.
        let status = Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("error")
            .arg("-y") // temp file will already exist, so allow overwriting
            .arg("-i")
            .arg(path.sandbox_path.as_os_str())
            .arg("-update")
            .arg("true")
            .arg("-vf")
            .arg(r"select=eq(n\,0)") // select frame zero
            .arg(temporary_png_file.path())
            .status()?;

        if !status.success() {
            anyhow::bail!("FFMpeg exited with status {:?}", status.code());
        }

        let src_image = ImageReader::open(&temporary_png_file)?.decode()?;

        let _ = self.thumbnailer.generate_all_thumbnails(path, src_image)?;

        Ok(())
    }
}
