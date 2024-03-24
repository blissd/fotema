// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::preview;
use crate::repo;
use crate::scanner;
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
        let pics = self.scan.scan_all()?;
        self.repo.add_all(&pics)?;
        self.repo.all()?;
        Ok(())
    }

    /// Gets all photos.
    pub fn all(&self) -> Result<Vec<repo::Picture>> {
        self.repo.all()
    }

    pub fn add_preview(&mut self, pic: &mut repo::Picture) -> Result<path::PathBuf> {
        let preview = self.prev.from_picture(pic.picture_id.id(), &pic.path);
        if let Ok(ref path) = preview {
            pic.square_preview_path = Some(path.clone());
            self.repo.add_preview(pic)?;
        }
        preview
    }
}
