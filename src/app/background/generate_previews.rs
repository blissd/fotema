// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use std::sync::{Arc, Mutex};
use photos_core::Result;
use rayon::prelude::*;

#[derive(Debug)]
pub enum GeneratePreviewsInput {
    Generate,
}

#[derive(Debug)]
pub enum GeneratePreviewsOutput {
    // Thumbnail generation has started for a given number of images.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Generated,

    // Thumbnail generation has completed
    Completed,

}

pub struct GeneratePreviews {
    previewer: photos_core::Previewer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: Arc<Mutex<photos_core::Repository>>,
}

impl GeneratePreviews {

    fn update_previews(&self, sender: &ComponentSender<Self>) -> Result<()> {
        let start = std::time::Instant::now();

        let mut pics: Vec<photos_core::repo::Picture> = self.repo
            .lock()
            .unwrap()
            .all()?
            .into_iter()
            .filter(|pic| !pic.square_preview_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        let pics_count = pics.len();
        sender.output(GeneratePreviewsOutput::Started(pics_count));

        // Process newer photos first.
        pics.reverse();

        pics.par_iter_mut()
            .map(|mut pic| {
                if let Err(e) = self.previewer.set_preview(&mut pic) {
                    println!("Failed setting preview: {:?}", e);
                }
                pic
            })
            .for_each(|pic| {
                let result = self.repo.lock().unwrap().add_preview(&pic);
                if let Err(e) = result {
                    println!("Failed add_preview: {:?}", e);
                } else if let Err(e) = sender.output(GeneratePreviewsOutput::Generated) {
                    println!("Failed sending GeneratePreviewsOutput::Generated: {:?}", e);
                }
            });

        println!("Generated {} previews in {} seconds.", pics_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(GeneratePreviewsOutput::Completed) {
            println!("Failed sending GeneratePreviewsOutput::Completed: {:?}", e);
        }

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
            }
        };
    }
}
