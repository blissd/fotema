// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::model::Visual;
use super::repo::Repository;
use crate::VisualId;
use anyhow::*;
use std::sync::{Arc, RwLock};

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
        let mut all = self
            .repo
            .all()?
            .into_iter()
            .map(|x| Arc::new(x))
            .collect::<Vec<Arc<Visual>>>();

        let mut index = self.index.write().unwrap();
        index.clear();
        index.append(&mut all);

        println!("Library has {} items", index.len());

        Ok(())
    }

    /// Gets a shared copy of visual library index.
    pub fn all(&self) -> Vec<Arc<Visual>> {
        let index = self.index.read().unwrap();
        index.clone()
    }

    /// Find an item by id.
    pub fn get(&self, visual_id: &VisualId) -> Option<Arc<Visual>> {
        let index = self.index.read().unwrap();
        index.iter().find(|&x| x.visual_id == *visual_id).cloned()
    }
}
