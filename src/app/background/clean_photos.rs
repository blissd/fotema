// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use rayon::prelude::*;
use anyhow::*;

#[derive(Debug)]
pub enum CleanPhotosInput {
    Start,
}

#[derive(Debug)]
pub enum CleanPhotosOutput {
    // Thumbnail generation has started for a given number of images.
    Started,

    // Thumbnail generation has completed
    Completed,

}

pub struct CleanPhotos {
    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl CleanPhotos {

    fn cleanup(&mut self, sender: &ComponentSender<Self>) -> Result<()> {

        let start = std::time::Instant::now();

        // Scrub pics from database if they no longer exist on the file system.
        let pics: Vec<fotema_core::photo::model::Picture> = self.repo.all()?;

        let pics_count = pics.len();

        if let Err(e) = sender.output(CleanPhotosOutput::Started){
            println!("Failed sending cleanup started: {:?}", e);
        }

        pics.par_iter()
            .for_each(|pic| {
                if !pic.path.exists() {
                    let result = self.repo.clone().remove(pic.picture_id);
                    if let Err(e) = result {
                        println!("Failed remove {}: {:?}", pic.picture_id, e);
                    } else {
                        println!("Removed {}", pic.picture_id);
                    }
                }
            });

        println!("Cleaned {} photos in {} seconds.", pics_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(CleanPhotosOutput::Completed) {
            println!("Failed sending CleanPhotosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for CleanPhotos {
    type Init = fotema_core::photo::Repository;
    type Input = CleanPhotosInput;
    type Output = CleanPhotosOutput;

    fn init(repo: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { repo }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            CleanPhotosInput::Start => {
                println!("Cleaning photos...");

                if let Err(e) = self.cleanup(&sender) {
                    println!("Failed to clean photos: {}", e);
                }
            }
        };
    }
}
