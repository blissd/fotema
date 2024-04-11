// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Error::*;
use crate::Result;
use chrono;
use chrono::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

/// A video on the local file system that has been scanned.
#[derive(Debug, Clone)]
pub struct Video {
    /// Full path to picture file.
    pub path: PathBuf,

    /// Metadata from the file system.
    pub fs: Option<FsMetadata>,
}

impl Video {
    /// Creates a new Picture for a given full path.
    pub fn new(path: PathBuf) -> Video {
        Video { path, fs: None }
    }

    pub fn created_at(&self) -> Option<chrono::DateTime<Utc>> {
        let fs_ts = self.fs.as_ref().and_then(|x| x.created_at);
        fs_ts
    }

    pub fn modified_at(&self) -> Option<chrono::DateTime<Utc>> {
        let fs_ts = self.fs.as_ref().and_then(|x| x.modified_at);
        fs_ts
    }
}

/// Metadata from the file system.
#[derive(Debug, Default, Clone)]
pub struct FsMetadata {
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
    pub file_size_bytes: Option<u64>,
}

impl FsMetadata {
    /// If any fields are present, then wrap self in an Option::Some. Otherwise, return None.
    pub fn to_option(self) -> Option<FsMetadata> {
        if self.created_at.is_some() || self.modified_at.is_some() || self.file_size_bytes.is_some()
        {
            Some(self)
        } else {
            None
        }
    }
}

/// Scans a file system for pictures.
#[derive(Debug, Clone)]
pub struct VideoScanner {
    /// File system path to scan.
    scan_base: PathBuf,
}

impl VideoScanner {
    pub fn build(scan_base: &Path) -> Result<VideoScanner> {
        fs::create_dir_all(scan_base).map_err(|e| ScannerError(e.to_string()))?;
        let scan_base = PathBuf::from(scan_base);
        Ok(VideoScanner { scan_base })
    }

    /// Scans all videos in the base directory for function `func` to visit.
    pub fn scan_all_visit<F>(&self, func: F)
    where
        F: FnMut(Video),
    {
        let suffixes = vec![String::from("mov"), String::from("mp4")];

        WalkDir::new(&self.scan_base)
            .into_iter()
            .flatten() // skip files we failed to read
            .filter(|x| x.path().is_file()) // only process files
            .filter(|x| {
                // only process supported image types
                let ext = x
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase());
                suffixes.contains(&ext.unwrap_or(String::from("not_a_video")))
            })
            .map(|x| self.scan_one(x.path())) // Get video info for path
            .flatten() // ignore any errors when reading videos
            .for_each(func); // visit
    }

    pub fn scan_all(&self) -> Result<Vec<Video>> {
        // Count of files in scan_base.
        // Note: no filtering here, so count could be greater than number of pictures.
        // Might want to use the same WalkDir logic in visit_all(...) to get exact count.
        let file_count = WalkDir::new(&self.scan_base).into_iter().count();
        let mut vids = Vec::with_capacity(file_count);
        self.scan_all_visit(|vid| vids.push(vid));
        Ok(vids)
    }

    pub fn scan_one(&self, path: &Path) -> Result<Video> {
        let file = fs::File::open(path).map_err(|e| ScannerError(e.to_string()))?;
        let mut fs = FsMetadata::default();

        fs.created_at = file
            .metadata()
            .ok()
            .and_then(|x| x.created().ok())
            .map(|x| Into::<DateTime<Utc>>::into(x));

        fs.modified_at = file
            .metadata()
            .ok()
            .and_then(|x| x.modified().ok())
            .map(|x| Into::<DateTime<Utc>>::into(x));

        fs.file_size_bytes = file.metadata().ok().map(|x| x.len());

        let mut vid = Video::new(PathBuf::from(path));
        vid.fs = fs.to_option();

        Ok(vid)
    }
}
