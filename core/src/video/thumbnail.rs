// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::thumbnail::Thumbnailer as PhotoThumbnailer;
use crate::video::model::{VideoExtra, VideoId};
use anyhow::*;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::result::Result::Ok;
use tempfile;

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
        let mut extra = VideoExtra::default();

        let thumbnail_path = {
            let file_name = format!("{}_{}x{}.png", video_id, EDGE, EDGE);
            self.base_path.join(file_name)
        };

        let result = self.compute_thumbnail(video_path, &thumbnail_path);

        if result.is_ok() {
            extra.thumbnail_path = Some(thumbnail_path.clone());
        } else {
            println!("Video thumbnail error: {:?}", result);
        }

        Ok(thumbnail_path)
    }

    fn compute_thumbnail(&self, video_path: &Path, thumbnail_path: &Path) -> Result<()> {
        if thumbnail_path.exists() {
            return Ok(());
        }

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
