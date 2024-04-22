// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::video::VideoId;

#[derive(Debug, Clone)]
pub struct Transcoder {
    /// Base path for storing transcoded videos
    base_path: PathBuf,
}

pub enum Error {
    Invalid(String),
}

impl Transcoder {
    pub fn new(base_path: &Path) -> Self {
        let base_path = PathBuf::from(base_path);
        Self { base_path }
    }

    /// Transcodes the video at 'path' and returns a path to the transcoded video.
    pub fn transcode(&self, video_id: VideoId, video_path: &Path) -> Result<PathBuf, Error> {
        let transcoded_path = {
            let file_name = format!("{}.mkv", video_id);
            self.base_path.join(file_name)
        };

        if transcoded_path.exists() {
            return Ok(PathBuf::from(transcoded_path));
        }

        println!("Transcoding video: {:?}", video_path);

        // FIXME can transcoding be reliably hardware accelerated?
        Command::new("ffmpeg")
            .arg("-loglevel")
            .arg("error")
            .arg("-i")
            .arg(video_path.as_os_str())
            .arg("-c:v")
            .arg("h264")
            .arg(transcoded_path.as_os_str())
            .status()
            .map_err(|e| Error::Invalid(format!("ffmpeg transcode result: {}", e)))?;

        Ok(PathBuf::from(transcoded_path))
    }
}
