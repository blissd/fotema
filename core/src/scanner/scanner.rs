// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::ScannedFile;
use crate::file_types;

use anyhow::*;
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
            .inspect(Self::inspect_err)
            .filter_map(|e| e.ok()) // skip files we failed to read
            .filter(|x| x.path().is_file()) // only process files
            .map(Self::to_scanned_file)
            .filter_map(|e| e.ok()) // ignore any errors when reading images
            .for_each(func); // visit
    }

    fn inspect_err(entry: &std::result::Result<DirEntry, walkdir::Error>) {
        let _ = entry
            .as_ref()
            .inspect_err(|e| error!("Failed walking: {:?}", e));
    }

    fn to_scanned_file(entry: DirEntry) -> Result<ScannedFile> {
        // only process supported image types
        let path = entry.path();
        let scanned_file = if file_types::is_supported_picture(path) {
            Ok(ScannedFile::Photo(path.into()))
        } else if file_types::is_supported_video(path) {
            Ok(ScannedFile::Video(path.into()))
        } else {
            Err(anyhow!("Not a picture or video: {:?}", path))
        };
        scanned_file
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
        let mut pics = Vec::new();
        self.scan_all_visit(|pic| pics.push(pic));
        Ok(pics)
    }
}
