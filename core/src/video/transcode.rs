// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::video::VideoId;

use tracing::{Level, event};

#[derive(Debug, Clone)]
pub struct Transcoder {
    /// Base path for storing transcoded videos
    base_path: PathBuf,
}

impl Transcoder {
    pub fn new(base_path: &Path) -> Self {
        let base_path = PathBuf::from(base_path).join("video_transcodes");
        let _ = std::fs::create_dir_all(&base_path);
        Self { base_path }
    }

    /// Transcodes the video at 'path' and returns a path to the transcoded video.
    pub fn transcode(&self, video_id: VideoId, video_path: &Path) -> Result<PathBuf> {
        let transcoded_path = {
            // Create a directory per 1000 videos
            let partition = (video_id.id() / 1000) as i32;
            let partition = format!("{:0>4}", partition);
            let file_name = format!("{}.mkv", video_id);
            self.base_path.join(partition).join(file_name)
        };

        transcode(video_path, &transcoded_path)?;

        Ok(transcoded_path)
    }
}

pub fn transcode(video_path: &Path, transcoded_path: &Path) -> Result<()> {
    if transcoded_path.exists() {
        return Ok(());
    } else if let Some(p) = transcoded_path.parent() {
        let _ = std::fs::create_dir_all(p);
    }

    event!(Level::DEBUG, "Transcoding video: {:?}", video_path);

    let temporary_transcoded_path = transcoded_path.with_extension("tmp.mkv");

    Command::new("ffmpeg")
        .arg("-y")
        .arg("-loglevel")
        .arg("error")
        .arg("-hwaccel")
        .arg("auto")
        .arg("-i")
        .arg(video_path.as_os_str())
        .arg("-c:a")
        .arg("copy")
        .arg("-c:v")
        .arg("h264")
        .arg(temporary_transcoded_path.as_os_str())
        .status()?;

    std::fs::rename(&temporary_transcoded_path, transcoded_path)?;

    Ok(())
}
