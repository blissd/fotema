// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use std::sync::{Arc, Mutex};
use photos_core::Result;
use rayon::prelude::*;

#[derive(Debug)]
pub enum CleanupInput {
    Start,
}

#[derive(Debug)]
pub enum CleanupOutput {
    // Thumbnail generation has started for a given number of images.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Cleaned,

    // Thumbnail generation has completed
    Completed,

}

pub struct Cleanup {
    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: Arc<Mutex<photos_core::Repository>>,
}

impl Cleanup {

    fn cleanup(&self, sender: &ComponentSender<Self>) -> Result<()> {

        let start = std::time::Instant::now();

        // Scrub pics from database if they no longer exist on the file system.
        let pics: Vec<photos_core::repo::Picture> = self.repo
            .lock()
            .unwrap()
            .all()?;

        let pics_count = pics.len();

        if let Err(e) = sender.output(CleanupOutput::Started(pics_count)){
            println!("Failed sending cleanup started: {:?}", e);
        }

        pics.par_iter()
            .for_each(|pic| {
                if !pic.path.exists() {
                    let result = self.repo.lock().unwrap().remove(pic.picture_id);
                    if let Err(e) = result {
                        println!("Failed remove {}: {:?}", pic.picture_id, e);
                    } else {
                        println!("Removed {}", pic.picture_id);
                    }
                }

                if let Err(e) = sender.output(CleanupOutput::Cleaned) {
                    println!("Failed sending CleanupOutput::Cleaned: {:?}", e);
                }
            });

        println!("Cleaned {} photos in {} seconds.", pics_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(CleanupOutput::Completed) {
            println!("Failed sending CleanupOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for Cleanup {
    type Init = Arc<Mutex<photos_core::Repository>>;
    type Input = CleanupInput;
    type Output = CleanupOutput;

    fn init(repo: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { repo }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            CleanupInput::Start => {
                println!("Cleanup...");

                if let Err(e) = self.cleanup(&sender) {
                    println!("Failed to update previews: {}", e);
                }
            }
        };
    }
}
