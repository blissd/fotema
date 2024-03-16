// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::repo;
use crate::scanner;
use crate::Result;

/// Aggregate API for the scanner and the repository.
#[derive(Debug)]
pub struct Controller {
    repo: repo::Repository,
    scan: scanner::Scanner,
}

/// Summary of a scan
#[derive(Debug)]
pub struct ScanSummary {
    /// Count of pictures scanned.
    success_count: u32,
    /// Count of pictures that could not be processed.
    error_count: u32,
}

impl Controller {
    pub fn new(repo: repo::Repository, scan: scanner::Scanner) -> Controller {
        Controller { repo, scan }
    }

    /// Scans all photos and adds them to the repository.
    pub fn scan(&self) -> Result<ScanSummary> {
        let mut summary = ScanSummary {
            success_count: 0,
            error_count: 0,
        };

        self.scan.visit_all(|pic| {
            let exif_date_time = pic.exif.and_then(|x| x.created_at);
            let fs_date_time = pic.fs.and_then(|x| x.created_at);
            let order_by_ts = exif_date_time.map(|d| d.to_utc()).or(fs_date_time);

            let pic = repo::Picture {
                relative_path: pic.relative_path,
                order_by_ts,
            };
            match self.repo.add(&pic) {
                Ok(_) => summary.success_count += 1,
                Err(_) => summary.error_count += 1,
            }
        });

        Ok(summary)
    }

    /// Gets all photos.
    pub fn all(&self) -> Result<Vec<repo::Picture>> {
        self.repo.all()
    }
}
