// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::transcode::Transcoder;
use super::Metadata;
use super::Thumbnailer;
use crate::video::model::{VideoExtra, VideoId};
use anyhow::*;
use std::path::Path;
use std::result::Result::Ok;

#[derive(Debug, Clone)]
pub struct Enricher {
    transcoder: Transcoder,
    thumbnailer: Thumbnailer,
}

impl Enricher {
    pub fn build(base_path: &Path, transcoder: Transcoder) -> Result<Self> {
        let thumbnailer = Thumbnailer::build(base_path)?;
        Ok(Self {
            transcoder,
            thumbnailer,
        })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub fn enrich(&self, video_id: &VideoId, video_path: &Path) -> Result<VideoExtra> {
        let mut extra = VideoExtra::default();

        if let Ok(metadata) = Metadata::from(video_path) {
            extra.stream_created_at = metadata.created_at;
            extra.stream_duration = metadata.duration;
            extra.video_codec = metadata.video_codec.clone();
            extra.content_id = metadata.content_id;

            // TODO split slow transcoding from fast enriching
            /*
            if metadata.video_codec.is_some_and(|x| x == "hevc") {
                extra.transcoded_path = self.transcoder.transcode(*video_id, video_path).ok();
            }
            */
        }

        extra.thumbnail_path = self.thumbnailer.thumbnail(video_id, video_path).ok();

        Ok(extra)
    }
}
