// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::ScannedFile;
use anyhow::*;
use chrono;
use chrono::prelude::*;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

// FIXME photos::Scanner and videos::Scanner are now broadly the same. Can they be consolidated?

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
        let picture_suffixes = vec![
            String::from("avif"),
            String::from("heic"), // not supported by image-rs
            String::from("jpeg"),
            String::from("jpg"),
            String::from("jxl"),
            String::from("png"),
            String::from("tiff"),
            String::from("webp"),
        ];

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
                picture_suffixes.contains(&ext.unwrap_or(String::from("not_an_image")))
            })
            .map(|x| self.scan_one(x.path())) // Get picture info for image path
            .flatten() // ignore any errors when reading images
            .for_each(func); // visit
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

#[cfg(test)]
mod tests {
    use super::*;

    fn picture_dir() -> PathBuf {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        test_data_dir
    }

    #[test]
    fn scan_all_visit() {
        let test_data_dir = picture_dir();
        let mut count = 0;
        let s = PhotoScanner::build(&test_data_dir).unwrap();
        s.scan_all_visit(|_| count += 1);
        assert_eq!(5, count);
    }

    #[test]
    fn scan_all() {
        let test_data_dir = picture_dir();
        let s = PhotoScanner::build(&test_data_dir).unwrap();
        let mut all = s.scan_all().unwrap();
        assert_eq!(5, all.len());
        all.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        assert!(all[0].path.ends_with("Dog.jpg"));
        assert!(all[1].path.ends_with("Frog.jpg"));
        assert!(all[2].path.ends_with("Kingfisher.jpg"));
        assert!(all[3].path.ends_with("Lavender.jpg"));
        assert!(all[4].path.ends_with("Sandow.jpg"));
    }

    #[test]
    fn scan_one() {
        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Sandow.jpg");

        let s = PhotoScanner::build(&test_data_dir).unwrap();
        let pic = s.scan_one(&test_file).unwrap();

        assert!(pic.path.to_str().unwrap().ends_with("Sandow.jpg"));
    }
}
