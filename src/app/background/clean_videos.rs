// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use rayon::prelude::*;
use anyhow::*;

#[derive(Debug)]
pub enum CleanVideosInput {
    Start,
}

#[derive(Debug)]
pub enum CleanVideosOutput {
    // Thumbnail generation has started for a given number of images.
    Started,

    // Thumbnail generation has completed
    Completed,

}

pub struct CleanVideos {
    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::video::Repository,
}

impl CleanVideos {

    fn cleanup(&mut self, sender: &ComponentSender<Self>) -> Result<()> {

        let start = std::time::Instant::now();

        // Scrub vids from database if they no longer exist on the file system.
        let vids: Vec<fotema_core::video::model::Video> = self.repo.all()?;

        let vids_count = vids.len();

        if let Err(e) = sender.output(CleanVideosOutput::Started){
            println!("Failed sending cleanup started: {:?}", e);
        }

        vids.par_iter()
            .for_each(|vid| {
                if !vid.path.exists() {
                    let result = self.repo.clone().remove(vid.video_id);
                    if let Err(e) = result {
                        println!("Failed remove {}: {:?}", vid.video_id, e);
                    } else {
                        println!("Removed {}", vid.video_id);
                    }
                }
            });

        println!("Cleaned {} videos in {} seconds.", vids_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(CleanVideosOutput::Completed) {
            println!("Failed sending CleanVideosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for CleanVideos {
    type Init = fotema_core::video::Repository;
    type Input = CleanVideosInput;
    type Output = CleanVideosOutput;

    fn init(repo: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { repo }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            CleanVideosInput::Start => {
                println!("Cleaning videos...");

                if let Err(e) = self.cleanup(&sender) {
                    println!("Failed to clean videos: {}", e);
                }
            }
        };
    }
}
