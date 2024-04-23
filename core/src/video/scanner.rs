// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::video::model::ScannedFile;
use anyhow::*;
use chrono::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

// FIXME photos::Scanner and videos::Scanner are now broadly the same. Can they be consolidated?

/// Scans a file system for videos.
#[derive(Debug, Clone)]
pub struct Scanner {
    /// File system path to scan.
    scan_base: PathBuf,
}

impl Scanner {
    pub fn build(scan_base: &Path) -> Result<Scanner> {
        fs::create_dir_all(scan_base)?;
        let scan_base = PathBuf::from(scan_base);
        Ok(Scanner { scan_base })
    }

    /// Scans all videos in the base directory for function `func` to visit.
    pub fn scan_all_visit<F>(&self, func: F)
    where
        F: FnMut(ScannedFile),
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

    pub fn scan_all(&self) -> Result<Vec<ScannedFile>> {
        // Count of files in scan_base.
        // Note: no filtering here, so count could be greater than number of pictures.
        // Might want to use the same WalkDir logic in visit_all(...) to get exact count.
        let file_count = WalkDir::new(&self.scan_base).into_iter().count();
        let mut vids = Vec::with_capacity(file_count);
        self.scan_all_visit(|vid| vids.push(vid));
        Ok(vids)
    }

    pub fn scan_one(&self, path: &Path) -> Result<ScannedFile> {
        let file = fs::File::open(path)?;

        let metadata = file.metadata()?;

        let fs_created_at = metadata.created().map(|x| Into::<DateTime<Utc>>::into(x))?;

        let fs_modified_at = metadata
            .modified()
            .map(|x| Into::<DateTime<Utc>>::into(x))?;

        let fs_file_size_bytes = metadata.len();

        let scanned = ScannedFile {
            path: PathBuf::from(path),
            fs_created_at,
            fs_modified_at,
            fs_file_size_bytes,
        };

        Ok(scanned)
    }
}
