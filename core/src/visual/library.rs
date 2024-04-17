// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::repo::{Repository, Visual};
use crate::Result;
use crate::VisualId;
use std::sync::{Arc, Mutex, RwLock};

/// Index of all images and photos in the library
#[derive(Clone)]
pub struct Library {
    repo: Repository,

    index: Arc<RwLock<Vec<Arc<Visual>>>>,
}

impl Library {
    pub fn new(repo: Repository) -> Library {
        Library {
            repo,
            index: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Reload all visual library items from database.
    pub fn refresh(&mut self) -> Result<()> {
        let all = self.repo.all()?;

        let mut index = self.index.write().unwrap();
        index.clear();
        for item in all {
            index.push(Arc::new(item));
        }

        Ok(())
    }

    /// Gets a shared copy of visual library index.
    pub fn all(&self) -> Vec<Arc<Visual>> {
        let index = self.index.read().unwrap();
        index.clone()
    }

    /// Find an item by id.
    pub fn get(&self, visual_id: VisualId) -> Option<Arc<Visual>> {
        let index = self.index.read().unwrap();
        index.iter().find(|&x| x.visual_id == visual_id).cloned()
    }
}
