// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::preview;
use crate::repo;
use crate::scanner;
use crate::Result;

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
        Ok(())
    }

    // Gets all photos that have a preview
    // TODO make a database query for this instead of filtering
    pub fn all_with_previews(&self) -> Result<Vec<repo::Picture>> {
        let pics = self.repo.all()?;
        let pics = pics
            .into_iter()
            .filter(|p| p.square_preview_path.is_some())
            .collect();
        Ok(pics)
    }

    /// Gets all photos.
    pub fn all(&self) -> Result<Vec<repo::Picture>> {
        self.repo.all()
    }

    pub fn update_previews(&mut self) -> Result<()> {
        // TODO make a database query to get pictures
        // that need a preview to be computed
        let pics = self.repo.all()?;
        let pics = pics
            .into_iter()
            .filter(|p| p.square_preview_path.is_none())
            .collect::<Vec<repo::Picture>>();

        for mut pic in pics {
            let result = self.prev.set_preview(&mut pic);
            if let Err(e) = result {
                println!("Failed set_preview: {:?}", e);
                continue;
            }
            self.repo.add_preview(&pic)?;
        }

        Ok(())
    }

    pub fn add_preview(&mut self, pic: &mut repo::Picture) -> Result<()> {
        self.prev.set_preview(pic)?;
        self.repo.add_preview(pic)
    }
}
