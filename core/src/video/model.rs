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

    /// Filesystem creation timestamp
    pub fs_created_at: DateTime<Utc>,

    /// Video stream metadata creation timestamp
    pub stream_created_at: Option<DateTime<Utc>>,

    // Video stream metadata duration
    pub stream_duration: Option<TimeDelta>,
}

/// A video on the local file system that has been scanned.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// Full path to picture file.
    pub path: PathBuf,

    pub fs_created_at: DateTime<Utc>,

    pub fs_modified_at: DateTime<Utc>,

    pub fs_file_size_bytes: u64,
}

/// Extra (non-filesystem) metadata for videos
#[derive(Debug, Clone, Default)]
pub struct VideoExtra {
    // Path to square thumbnail file
    pub thumbnail_path: Option<PathBuf>,

    // Creation timestamp from stream metadata
    pub stream_created_at: Option<DateTime<Utc>>,

    // Video duration in stream metadata
    pub stream_duration: Option<TimeDelta>,

    // Video codec
    pub video_codec: Option<String>,

    // iOS id for linking a video with a photo
    pub content_id: Option<String>,

    // Path to transcoded video
    pub transcoded_path: Option<PathBuf>,
}
