// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, TimeDelta, Utc};
use std::fmt::Display;
use std::path::PathBuf;

/// Database ID of video
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoId(i64);

impl VideoId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// FIXME replace this with a To/From SQL implementation.
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for VideoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Video in database
#[derive(Debug, Clone)]
pub struct Video {
    /// Full path from library root.
    pub path: PathBuf,

    /// Database primary key for video
    pub video_id: VideoId,

    /// Full path to square preview image
    pub thumbnail_path: Option<PathBuf>,

    /// Time ordering
    pub ordering_ts: DateTime<Utc>,

    /// Video stream metadata duration
    pub stream_duration: Option<TimeDelta>,

    /// Path to transcoded video
    pub transcoded_path: Option<PathBuf>,

    /// Video codec
    pub video_codec: Option<String>,
}

/// A video on the local file system that has been scanned.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// Full path to picture file.
    pub path: PathBuf,

    pub fs_created_at: Option<DateTime<Utc>>,

    pub fs_modified_at: Option<DateTime<Utc>>,

    pub fs_file_size_bytes: u64,
}

#[derive(Debug, Default, Clone)]
pub struct Metadata {
    pub created_at: Option<DateTime<Utc>>,

    pub width: Option<u64>, // 64?

    pub height: Option<u64>,

    pub duration: Option<TimeDelta>,

    pub container_format: Option<String>,

    pub video_codec: Option<String>,

    pub audio_codec: Option<String>,

    pub content_id: Option<String>, // TODO make this a non-string type

    // Rotation of video in degrees.
    // Should be 90, 180, 270, or the negative of those.
    pub rotation: Option<i32>,
}
