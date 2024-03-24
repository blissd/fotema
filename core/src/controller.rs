// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::preview;
use crate::repo;
use crate::scanner;
use crate::Error::*;
use crate::Result;
use std::path;

/// Aggregate API for the scanner and the repository.
#[derive(Debug)]
pub struct Controller {
    scan: scanner::Scanner,
    repo: repo::Repository,
    prev: preview::Previewer,
}

impl Controller {
    pub fn new(
        scan: scanner::Scanner,
        repo: repo::Repository,
        prev: preview::Previewer,
    ) -> Controller {
        Controller { scan, repo, prev }
    }

    /// Scans all photos and adds them to the repository.
    pub fn scan(&mut self) -> Result<()> {
        fn as_repo_pic(pic: scanner::Picture) -> repo::Picture {
            let exif_date_time = pic.exif.and_then(|x| x.created_at);
            let fs_date_time = pic.fs.and_then(|x| x.created_at);
            let order_by_ts = exif_date_time.map(|d| d.to_utc()).or(fs_date_time);

            repo::Picture {
                path: pic.path,
                picture_id: None,
                square_preview_path: None,
                order_by_ts,
            }
        }

        let all_pics = match self.scan.scan_all() {
            Ok(pics) => {
                let pics = pics.into_iter().map(|p| as_repo_pic(p)).collect();
                self.repo.add_all(&pics)?;
                self.repo.all()?
            }
            Err(e) => {
                println!("Failed: {:?}", e);
                return Err(e);
            }
        };

        Ok(())
    }

    /// Gets all photos.
    pub fn all(&self) -> Result<Vec<repo::Picture>> {
        self.repo.all()
    }

    pub fn add_preview(&mut self, pic: &mut repo::Picture) -> Result<path::PathBuf> {
        let preview = self
            .prev
            .from_picture(pic.picture_id.unwrap().id(), &pic.path);
        if let Ok(ref path) = preview {
            pic.square_preview_path = Some(path.clone());
            self.repo.add_preview(pic)?;
        }
        preview
    }
}
