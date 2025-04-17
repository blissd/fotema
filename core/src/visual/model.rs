// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fmt::Display;
use std::path::PathBuf;

use crate::photo::model::Orientation;
use crate::thumbnailify;
use crate::{PictureId, VideoId, YearMonth};

use chrono::*;

use h3o::LatLng;

pub use crate::photo::model::Orientation as PictureOrientation;

/// Database ID of a visual item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualId(String);

impl VisualId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn id(&self) -> &String {
        &self.0
    }
}

impl Display for VisualId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A visual artefact, such as a photo or a video (or in some cases both at once).
#[derive(Debug, Clone)]
pub struct Visual {
    /// Full path from library root.
    pub visual_id: VisualId,

    // Path to parent directory
    pub parent_path: PathBuf,

    /// Path to thumbnail. If both a picture and a video are present, then this will
    /// be the picture thumbnail path.
    pub thumbnail_path: Option<PathBuf>,

    pub video_id: Option<VideoId>,

    pub video_path: Option<PathBuf>,

    pub video_host_path: Option<PathBuf>,

    // Transcoded version of video_path of video_codec is not supported.
    pub video_transcoded_path: Option<PathBuf>,

    // Duration of video content
    pub video_duration: Option<TimeDelta>,

    // Rotation of video content. Derived from degrees.
    pub video_orientation: Option<Orientation>,

    pub picture_id: Option<PictureId>,

    pub picture_path: Option<PathBuf>,

    pub picture_host_path: Option<PathBuf>,

    pub picture_orientation: Option<Orientation>,

    pub motion_photo_video_path: Option<PathBuf>,

    /// Best candidate for ordering visual items. With a final fallback of the current timestamp.
    pub ordering_ts: DateTime<Utc>,

    // Is this a selfie?
    pub is_selfie: Option<bool>,

    // Is this an iOS live photo?
    pub is_live_photo: bool,

    // Does the video_code require the video is transcoded?
    pub is_transcode_required: Option<bool>,

    // Where photo was taken
    pub location: Option<LatLng>,
}

impl Visual {
    pub fn path(&self) -> Option<&PathBuf> {
        self.picture_path.as_ref().or(self.video_path.as_ref())
    }

    // FIXME not an option! One must always be present.
    pub fn host_path(&self) -> Option<&PathBuf> {
        self.picture_host_path
            .as_ref()
            .or(self.video_host_path.as_ref())
    }

    pub fn thumbnail_hash(&self) -> String {
        thumbnailify::compute_hash_for_path(&self.host_path().expect("Must have host path!"))
    }

    pub fn is_selfie(&self) -> bool {
        self.is_selfie.is_some_and(|x| x)
    }

    pub fn is_motion_photo(&self) -> bool {
        self.is_live_photo
    }

    pub fn is_photo_only(&self) -> bool {
        self.picture_id.is_some() && self.video_id.is_none() && !self.is_live_photo
    }

    pub fn is_video_only(&self) -> bool {
        self.picture_id.is_none() && self.video_id.is_some()
    }

    pub fn year(&self) -> u32 {
        self.ordering_ts.date_naive().year_ce().1
    }

    pub fn year_month(&self) -> YearMonth {
        let date = self.ordering_ts.date_naive();
        let year = date.year();
        let month = date.month();
        let month = chrono::Month::try_from(u8::try_from(month).unwrap()).unwrap();
        YearMonth { year, month }
    }

    // TODO should really just compute this in photo_info.rs
    pub fn folder_name(&self) -> Option<String> {
        self.parent_path
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
    }
}
