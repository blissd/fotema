// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::FileInfo;
use super::ScannedFile;

use anyhow::*;
use chrono;
use chrono::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use tracing::error;
use walkdir::{DirEntry, WalkDir};

/// Scans a file system for pictures.
#[derive(Debug, Clone)]
pub struct Scanner {
    /// File system path to scan.
    scan_base: PathBuf,
}

impl Scanner {
    const PICTURES_SUFFIXES: [&str; 11] = [
        "avif", "exr", "heic", "jpeg", "jpg", "jxl", "png", "qoi", "tiff", "webp", "gif",
    ];

    const VIDEO_SUFFIXES: [&str; 5] = ["m4v", "mov", "mp4", "avi", "mkv"];

    pub fn build(scan_base: &Path) -> Result<Self> {
        fs::create_dir_all(scan_base)?;
        let scan_base = PathBuf::from(scan_base);
        Ok(Self { scan_base })
    }

    /// Scans all pictures in the base directory for function `func` to visit.
    pub fn scan_all_visit<F>(&self, func: F)
    where
        F: FnMut(ScannedFile),
    {
        WalkDir::new(&self.scan_base)
            .into_iter()
            .filter_entry(|e| !Scanner::is_hidden(e))
            .inspect(|x| {
                let _ = x
                    .as_ref()
                    .inspect_err(|e| error!("Failed walking: {:?}", e));
            })
            .flatten() // skip files we failed to read
            .filter(|x| x.path().is_file()) // only process files
            .map(|x| {
                // only process supported image types
                let ext = x
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase())
                    .unwrap_or(String::from("unknown"));

                let scanned_file = if Self::PICTURES_SUFFIXES.contains(&ext.as_ref()) {
                    self.scan_one(x.path())
                        .map(|info| ScannedFile::Photo(info))
                        .map_err(|e| {
                            error!("Failed scanning picture: {:?}", e);
                            e
                        })
                } else if Self::VIDEO_SUFFIXES.contains(&ext.as_ref()) {
                    self.scan_one(x.path())
                        .map(|info| ScannedFile::Video(info))
                        .map_err(|e| {
                            error!("Failed scanning video: {:?}", e);
                            e
                        })
                } else {
                    Err(anyhow!("Not a picture or video: {:?}", x))
                };
                scanned_file
            })
            .flatten() // ignore any errors when reading images
            .for_each(func); // visit
    }

    fn is_hidden(entry: &DirEntry) -> bool {
        entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with("."))
            .unwrap_or(false)
    }

    pub fn scan_all(&self) -> Result<Vec<ScannedFile>> {
        // Count of files in scan_base.
        // Note: no filtering here, so count could be greater than number of pictures.
        // Might want to use the same WalkDir logic in visit_all(...) to get exact count.
        let file_count = WalkDir::new(&self.scan_base).into_iter().count();
        let mut pics = Vec::with_capacity(file_count);
        self.scan_all_visit(|pic| pics.push(pic));
        Ok(pics)
    }

    pub fn scan_one(&self, path: &Path) -> Result<FileInfo> {
        let metadata = fs::metadata(path)?;

        let fs_created_at = metadata.created().map(Into::<DateTime<Utc>>::into).ok();

        let fs_modified_at = metadata.modified().map(Into::<DateTime<Utc>>::into).ok();

        let fs_file_size_bytes = metadata.len();

        let scanned = FileInfo {
            path: PathBuf::from(path),
            fs_created_at,
            fs_modified_at,
            fs_file_size_bytes,
        };

        Ok(scanned)
    }
}
