// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, Utc};
use std::path::PathBuf;

/// A file that has been scanned.
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// Full path to picture file.
    pub path: PathBuf,

    pub fs_created_at: Option<DateTime<Utc>>,

    pub fs_modified_at: Option<DateTime<Utc>>,

    pub fs_file_size_bytes: u64,
}

#[derive(Debug, Clone)]
pub enum ScannedFile {
    Photo(FileInfo),
    Video(FileInfo),
}
