// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::YearMonth;
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset, Utc};
use std::fmt::Display;
use std::path::PathBuf;

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

#[derive(Debug, Clone, Default)]
pub struct PhotoExtra {
    // Path to square thumbnail file
    pub thumbnail_path: Option<PathBuf>,

    pub exif_created_at: Option<DateTime<FixedOffset>>,

    pub exif_modified_at: Option<DateTime<FixedOffset>>,

    /// On iPhone the lens model tells you if it was the front or back camera.
    pub exif_lens_model: Option<String>,
}

impl PhotoExtra {
    pub fn is_selfie(&self) -> bool {
        self.exif_lens_model
            .as_ref()
            .is_some_and(|x| x.contains("front"))
    }
}
