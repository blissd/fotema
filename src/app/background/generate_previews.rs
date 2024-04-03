// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use std::sync::{Arc, Mutex};
use photos_core::Result;
use photos_core::repo::PictureId;
use std::path::PathBuf;

#[derive(Debug)]
pub enum GeneratePreviewsInput {
    Generate,
}

#[derive(Debug)]
pub enum GeneratePreviewsOutput {
    PreviewsGenerated,
    PreviewUpdated(PictureId, Option<PathBuf>),
}

pub struct GeneratePreviews {
    previewer: photos_core::Previewer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: Arc<Mutex<photos_core::Repository>>,
}

impl GeneratePreviews {

    fn update_previews(&self, sender: &ComponentSender<Self>) -> Result<()> {
        let start = std::time::Instant::now();

        let mut pics = self.repo.lock().unwrap().all()?;
        let pics_count = pics.len();

        // Process newer photos first.
        pics.reverse();

        for mut pic in pics {
            let result = self.previewer.set_preview(&mut pic);
            if let Err(e) = result {
                println!("Failed set_preview: {:?}", e);
                continue;
            }

            let result = self.repo.lock().unwrap().add_preview(&pic);
            if let Err(e) = result {
                println!("Failed add_preview: {:?}", e);
                continue;
            }

            sender.output(GeneratePreviewsOutput::PreviewUpdated(pic.picture_id, pic.square_preview_path));
        }

        println!("Generated {} previews in {} seconds.", pics_count, start.elapsed().as_secs());

        Ok(())
    }
}

impl Worker for GeneratePreviews {
    type Init = (photos_core::Previewer, Arc<Mutex<photos_core::Repository>>);
    type Input = GeneratePreviewsInput;
    type Output = GeneratePreviewsOutput;

    fn init((previewer, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { previewer, repo }
    }

    fn update(&mut self, msg: GeneratePreviewsInput, sender: ComponentSender<Self>) {
        match msg {
            GeneratePreviewsInput::Generate => {
                println!("Generating previews...");

                if let Err(e) = self.update_previews(&sender) {
                    println!("Failed to update previews: {}", e);
                }

                if let Err(e) = sender.output(GeneratePreviewsOutput::PreviewsGenerated) {
                    println!("Failed notifying previews generated: {:?}", e);
                }
            }
        };
    }
}
