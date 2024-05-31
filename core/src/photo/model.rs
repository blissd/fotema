// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::YearMonth;
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use std::fmt::Display;
use std::path::PathBuf;
use strum::{AsRefStr, EnumIter};

/// Database ID of picture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureId(i64);

impl PictureId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// FIXME replace this with a To/From SQL implementation.
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for PictureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A picture in the repository
#[derive(Debug, Clone)]
pub struct Picture {
    /// Full path from picture library root.
    pub path: PathBuf,

    /// Database primary key for picture
    pub picture_id: PictureId,

    /// Full path to square preview image
    pub thumbnail_path: Option<PathBuf>,

    /// Creation timestamp from file system.
    pub fs_created_at: DateTime<Utc>,

    /// Creation timestamp from EXIF metadata.
    pub exif_created_at: Option<DateTime<Utc>>,

    /// Creation timestamp from EXIF metadata.
    pub exif_modified_at: Option<DateTime<Utc>>,

    /// Was picture taken with front camera?
    pub is_selfie: Option<bool>,
}

impl Picture {
    pub fn parent_path(&self) -> Option<PathBuf> {
        self.path.parent().map(|x| PathBuf::from(x))
    }

    pub fn folder_name(&self) -> Option<String> {
        self.path
            .parent()
            .and_then(|x| x.file_name())
            .map(|x| x.to_string_lossy().to_string())
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.exif_created_at.unwrap_or(self.fs_created_at)
    }

    pub fn year(&self) -> u32 {
        self.created_at().date_naive().year_ce().1
    }

    pub fn year_month(&self) -> YearMonth {
        let date = self.created_at().date_naive();
        let year = date.year();
        let month = date.month();
        let month = chrono::Month::try_from(u8::try_from(month).unwrap()).unwrap();
        YearMonth { year, month }
    }

    pub fn date(&self) -> chrono::NaiveDate {
        self.created_at().date_naive()
    }
}

// scanner

/// A picture on the local file system that has been scanned.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// Full path to picture file.
    pub path: PathBuf,

    pub fs_created_at: DateTime<Utc>,

    pub fs_modified_at: DateTime<Utc>,

    pub fs_file_size_bytes: u64,
}

/// Extra (non-filesystem) metadata for videos

// EXIF data can include an orientation, which is a number from 1 to 8 that describes
// the rotation/flipping to apply.
//
// 1 = 0 degrees: the correct orientation, no adjustment is required.
// 2 = 0 degrees, mirrored: image has been flipped back-to-front.
// 3 = 180 degrees: image is upside down.
// 4 = 180 degrees, mirrored: image has been flipped back-to-front and is upside down.
// 5 = 90 degrees: image has been flipped back-to-front and is on its side.
// 6 = 90 degrees, mirrored: image is on its side.
// 7 = 270 degrees: image has been flipped back-to-front and is on its far side.
// 8 = 270 degrees, mirrored: image is on its far side.
//
// The Orientation enum describes where the top of the image should point and if
// it should be mirrored (flipped on the X axis).
//
// NOTE: these enum names will be used in style.css to apply the rotation and mirroring.
//
// TODO this is also used by videos so move to a common place.

#[derive(Debug, Clone, Copy, AsRefStr, EnumIter)]
pub enum Orientation {
    // no rotation, no flip
    North = 1,

    // no rotation, flip on X axis
    NorthMirrored = 2,

    // Rotate 180, no flip
    South = 3,

    // Rotate 180, flip X axis
    SouthMirrored = 4,

    // Rotate 270 (90 anti-clockwise), flip X axis,
    WestMirrored = 5,

    // Rotate 270 clock-wise (90 anti-clockwise), no flip
    West = 6,

    // Rotate 90 clock-wise, flip X axis
    EastMirrored = 7,

    // Rotate 90 clock-wise, no flip
    East = 8,
}

impl Orientation {
    pub fn from_degrees(degrees: i32) -> Self {
        match degrees {
            0 => Orientation::North,
            90 | -270 => Orientation::East,
            180 | -180 => Orientation::South,
            -90 | 270 => Orientation::West,
            _ => Self::default(),
        }
    }
}

impl Default for Orientation {
    fn default() -> Self {
        Self::North
    }
}

impl From<u32> for Orientation {
    fn from(number: u32) -> Self {
        match number {
            1 => Orientation::North,
            2 => Orientation::NorthMirrored,
            3 => Orientation::South,
            4 => Orientation::SouthMirrored,
            5 => Orientation::WestMirrored,
            6 => Orientation::West,
            7 => Orientation::EastMirrored,
            8 => Orientation::East,
            _ => Self::default(),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Metadata {
    pub created_at: Option<DateTime<FixedOffset>>,

    pub modified_at: Option<DateTime<FixedOffset>>,

    /// On iPhone the lens model tells you if it was the front or back camera.
    pub lens_model: Option<String>,

    // iOS id for linking a video with a photo
    pub content_id: Option<String>,

    // EXIF orientation.
    // Some images... annoyingly... needs a rotation and mirror transformation applied
    // to display correctly.
    pub orientation: Option<Orientation>,
}

impl Metadata {
    pub fn is_selfie(&self) -> bool {
        self.lens_model
            .as_ref()
            .is_some_and(|x| x.contains("front"))
    }
}

/// A video extracted from a motion photo
#[derive(Debug, Clone)]
pub struct MotionPhotoVideo {
    pub path: PathBuf,
    pub duration: Option<TimeDelta>,
    pub video_codec: Option<String>,
    pub transcoded_path: Option<PathBuf>,

    // Rotation of video in degrees.
    // Should be 90, 180, 270, or the negative of those.
    pub rotation: Option<i32>,
}
