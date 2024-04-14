// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::video::model::{VideoExtra, VideoId};
use crate::Error::*;
use crate::Result;
use chrono::{DateTime, TimeDelta, Utc};
use image::io::Reader as ImageReader;
use image::DynamicImage;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile;

const EDGE: u32 = 200;

#[derive(Debug, Clone)]
pub struct Thumbnailer {
    base_path: PathBuf,
}

impl Thumbnailer {
    pub fn build(base_path: &Path) -> Result<Self> {
        let base_path = PathBuf::from(base_path);
        std::fs::create_dir_all(base_path.join("square"))
            .map_err(|e| RepositoryError(e.to_string()))?;
        Ok(Self { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub fn get_extra(&self, video_id: &VideoId, video_path: &Path) -> Result<VideoExtra> {
        let mut extra = VideoExtra::default();

        let thumbnail_path = {
            let file_name = format!("{}_{}x{}.png", video_id, EDGE, EDGE);
            self.base_path.join(file_name)
        };

        self.compute_thumbnail(video_path, &thumbnail_path)
            .map_err(|e| PreviewError(format!("save video thumbnail: {}", e)))?;

        extra.thumbnail_path = Some(thumbnail_path.clone());

        if let Ok((created_at, duration)) = Thumbnailer::get_stream_metadata(video_path) {
            extra.stream_created_at = created_at;
            extra.stream_duration = duration;
        }

        Ok(extra)
    }

    /// Use ffprobe to extract creation timestamp and video duration.
    fn get_stream_metadata(
        video_path: &Path,
    ) -> Result<(Option<DateTime<Utc>>, Option<TimeDelta>)> {
        // ffprobe is part of the ffmpeg-full flatpak extension
        let output = Command::new("/usr/bin/ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-i")
            .arg(video_path.as_os_str())
            .arg("-print_format")
            .arg("json")
            .arg("-select_streams")
            .arg("v:0")
            .arg("-show_entries")
            .arg("format=duration:stream_tags=creation_time")
            .output()
            .map_err(|e| PreviewError(format!("ffprobe result: {}", e)))?;

        let v: Value = serde_json::from_slice(output.stdout.as_slice())
            .map_err(|e| PreviewError(format!("parse ffprobe json: {}", e)))?;

        let creation_time = v["streams"][0]["tags"]["creation_time"].as_str();
        let creation_time = creation_time.and_then(|x| {
            let dt = DateTime::parse_from_rfc3339(x).ok();
            dt.map(|y| y.to_utc())
        });

        let time_delta = v["format"]["duration"].as_str(); // seconds with decimal
        let time_delta = time_delta.and_then(|x| {
            let fractional_secs = x.parse::<f64>();
            let millis = fractional_secs.map(|s| s * 1000.0).ok();
            millis.and_then(|m| TimeDelta::try_milliseconds(m as i64))
        });

        Ok((creation_time, time_delta))
    }

    fn compute_thumbnail(&self, video_path: &Path, thumbnail_path: &Path) -> Result<()> {
        if thumbnail_path.exists() {
            return Ok(());
        }

        let temporary_png_file = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(|e| PreviewError(format!("Temp file: {}", e)))?;

        // ffmpeg is installed as a flatpak extension.
        // ffmpeg command will extract the first frame and save it as a PNG file.
        Command::new("/usr/bin/ffmpeg")
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
            .status()
            .map_err(|e| PreviewError(format!("ffmpeg result: {}", e)))?;

        let thumbnail = self.standard_thumbnail(temporary_png_file.path())?;

        thumbnail
            .save(thumbnail_path)
            .or_else(|e| {
                let _ = std::fs::remove_file(&thumbnail_path);
                Err(e) // don't lose original error
            })
            .map_err(|e| PreviewError(format!("save video thumbnail: {}", e)))?;

        Ok(())
    }

    // FIXME copy-and-paste from photo thumbnailer
    fn standard_thumbnail(&self, path: &Path) -> Result<DynamicImage> {
        let img = ImageReader::open(path)
            .map_err(|e| PreviewError(format!("image open: {}", e)))?
            .decode()
            .map_err(|e| PreviewError(format!("image decode: {}", e)))?;

        let img = if img.width() == img.height() && img.width() == EDGE {
            return Ok(img);
        } else if img.width() == img.height() {
            img
        } else if img.width() < img.height() {
            let h = (img.height() - img.width()) / 2;
            img.crop_imm(0, h, img.width(), img.width())
        } else {
            let w = (img.width() - img.height()) / 2;
            img.crop_imm(w, 0, img.height(), img.height())
        };

        let img = img.thumbnail(EDGE, EDGE);
        Ok(img)
    }
}
