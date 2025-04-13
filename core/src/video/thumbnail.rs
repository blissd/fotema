// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::thumbnail::Thumbnailer as PhotoThumbnailer;
use crate::thumbnailify;
use crate::video::model::VideoId;
use anyhow::*;
use image::ImageReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::result::Result::Ok;
use tempfile;
use tracing::{debug, error};

const EDGE: u32 = 200;

/// Thumbnail operations for videos.
#[derive(Debug, Clone)]
pub struct Thumbnailer {
    base_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(base_path: &Path) -> Result<Thumbnailer> {
        Ok(Thumbnailer {
            base_path: base_path.into(),
        })
    }

    /// Computes a preview for a video
    pub fn thumbnail(&self, host_path: &Path, sandbox_path: &Path) -> Result<PathBuf> {
        // Extract first frame of video for thumbnail
        let temporary_png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        // ffmpeg command will extract the first frame and save it as a PNG file.
        Command::new("ffmpeg")
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

        let src_image = ImageReader::open(&temporary_png_file)?.decode()?;

        let thumb_path = thumbnailify::generate_thumbnail(
            &self.base_path,
            host_path,
            sandbox_path,
            thumbnailify::ThumbnailSize::XLarge,
            src_image,
        )?;

        Ok(thumb_path)
    }
}
