// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::SharedState;
use anyhow::*;
use fotema_core::Visual;
use fotema_core::visual::Repository;
use relm4::Worker;
use relm4::prelude::*;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug)]
pub enum LoadLibraryInput {
    Refresh,
}

#[derive(Debug)]
pub enum LoadLibraryOutput {
    Done,
}

pub struct LoadLibrary {
    repo: Repository,
    state: SharedState,
}

impl Worker for LoadLibrary {
    type Init = (Repository, SharedState);
    type Input = LoadLibraryInput;
    type Output = LoadLibraryOutput;

    fn init((repo, state): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { repo, state }
    }

    fn update(&mut self, msg: LoadLibraryInput, sender: ComponentSender<Self>) {
        match msg {
            LoadLibraryInput::Refresh => {
                let result = self.load();

                if let Err(e) = result {
                    error!("Failed load library with: {}", e);
                }

                let _ = sender.output(LoadLibraryOutput::Done);
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
            .map(Arc::new)
            .collect::<Vec<Arc<Visual>>>();

        info!("Loaded {} visual items", all.len());

        let mut index = self.state.write();
        index.clear();
        index.append(&mut all);
        Ok(())
    }
}
