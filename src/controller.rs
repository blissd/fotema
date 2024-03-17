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
#[derive(Debug, Default)]
pub struct ScanSummary {
    /// Count of pictures scanned.
    success_count: usize,
}

impl Controller {
    pub fn new(repo: repo::Repository, scan: scanner::Scanner) -> Controller {
        Controller { repo, scan }
    }

    /// Scans all photos and adds them to the repository.
    pub fn scan(&mut self) -> Result<ScanSummary> {
        fn as_repo_pic(pic: scanner::Picture) -> repo::Picture {
            let exif_date_time = pic.exif.and_then(|x| x.created_at);
            let fs_date_time = pic.fs.and_then(|x| x.created_at);
            let order_by_ts = exif_date_time.map(|d| d.to_utc()).or(fs_date_time);

            repo::Picture {
                relative_path: pic.relative_path,
                order_by_ts,
            }
        }

        match self.scan.scan_all() {
            Ok(pics) => {
                // TODO can an interator be passes to add_all instead of a vector?
                let pics = pics.into_iter().map(|p| as_repo_pic(p)).collect();
                self.repo.add_all(&pics)?;
                Ok(ScanSummary {
                    success_count: pics.len(),
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Gets all photos.
    pub fn all(&self) -> Result<Vec<repo::Picture>> {
        self.repo.all()
    }
}
