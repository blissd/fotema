// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::ScannedFile;

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
                    Ok(ScannedFile::Photo(x.path().into()))
                } else if Self::VIDEO_SUFFIXES.contains(&ext.as_ref()) {
                    Ok(ScannedFile::Video(x.path().into()))
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

    /*fn is_picture(path: &Path) -> bool {
        let Some(path_ext) = path.extension() {
            for ext in Self::PICTURES_SUFFIXES {
                if ext.eq_ignore_ascii_case(path.ex)
            }
        } else false
    }*/

    pub fn scan_all(&self) -> Result<Vec<ScannedFile>> {
        // Count of files in scan_base.
        // Note: no filtering here, so count could be greater than number of pictures.
        // Might want to use the same WalkDir logic in visit_all(...) to get exact count.
        let mut pics = Vec::new();
        self.scan_all_visit(|pic| pics.push(pic));
        Ok(pics)
    }
}
