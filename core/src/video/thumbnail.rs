// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::thumbnail::Thumbnailer as PhotoThumbnailer;
use crate::video::model::VideoId;
use anyhow::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::result::Result::Ok;
use tempfile;
use tracing::{event, Level};

const EDGE: u32 = 200;

/// Thumbnail operations for videos.
#[derive(Debug, Clone)]
pub struct Thumbnailer {
    base_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(base_path: &Path) -> Result<Thumbnailer> {
        let base_path = PathBuf::from(base_path).join("video_thumbnails");
        std::fs::create_dir_all(&base_path)?;

        Ok(Thumbnailer { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub fn thumbnail(&self, video_id: &VideoId, video_path: &Path) -> Result<PathBuf> {
        let thumbnail_path = {
            // Create a directory per 1000 thumbnails
            let partition = (video_id.id() / 1000) as i32;
            let partition = format!("{:0>4}", partition);
            let file_name = format!("{}_{}x{}.png", video_id, EDGE, EDGE);
            self.base_path.join(partition).join(file_name)
        };

        if thumbnail_path.exists() {
            return Ok(thumbnail_path);
        } else if let Some(p) = thumbnail_path.parent() {
            let _ = std::fs::create_dir_all(p);
        }

        event!(Level::DEBUG, "Standard thumbnail: {:?}", video_path);

        self.compute_thumbnail(video_path, &thumbnail_path)
            .map(|_| thumbnail_path)
            .inspect_err(|e| event!(Level::ERROR, "Video thumbnail error: {:?}", e))
    }

    fn compute_thumbnail(&self, video_path: &Path, thumbnail_path: &Path) -> Result<()> {
        let temporary_png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        // ffmpeg command will extract the first frame and save it as a PNG file.
        Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("error")
            .arg("-y") // temp file will already exist, so allow overwriting
            .arg("-i")
            .arg(video_path.as_os_str())
            .arg("-update")
            .arg("true")
            .arg("-vf")
            .arg(r"select=eq(n\,0)") // select frame zero
            .arg(temporary_png_file.path())
            .status()?;

        PhotoThumbnailer::fast_thumbnail(temporary_png_file.path(), thumbnail_path)
    }
}
