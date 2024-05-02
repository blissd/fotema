// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use crate::app::SharedState;
use fotema_core::visual::Repository;
use fotema_core::Visual;
use std::sync::Arc;
use anyhow::*;

#[derive(Debug)]
pub enum LoadLibraryInput {
    Refresh,
}

pub struct LoadLibrary {
    repo: Repository,
    state: SharedState,
}

impl Worker for LoadLibrary {
    type Init = (Repository, SharedState);
    type Input = LoadLibraryInput;
    type Output = ();

    fn init((repo, state): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { repo, state }
    }

    fn update(&mut self, msg: LoadLibraryInput, _sender: ComponentSender<Self>) {
        match msg {
            LoadLibraryInput::Refresh => {
                let result = self.load();

                if let Err(e) = result {
                    println!("Failed load library with: {}", e);
                }
            }
        };
    }
}

impl LoadLibrary {
    fn load(&self) -> Result<()> {
        let mut all = self
            .repo
            .all()?
            .into_iter()
            .map(|x| Arc::new(x))
            .collect::<Vec<Arc<Visual>>>();

        let mut index = self.state.write();
        index.clear();
        index.append(&mut all);
        Ok(())
    }
}
