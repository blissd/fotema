// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use rayon::prelude::*;
use anyhow::Result;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{debug, error, info};

#[derive(Debug)]
pub enum PhotoCleanInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoCleanOutput {
    // Thumbnail generation has started for a given number of images.
    Started,

    // Thumbnail generation has completed
    Completed(usize),

}

pub struct PhotoClean {
    // Stop flag
    stop: Arc<AtomicBool>,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl PhotoClean {

    fn cleanup(&mut self, sender: &ComponentSender<Self>) -> Result<()> {

        let start = std::time::Instant::now();

        // Scrub pics from database if they no longer exist on the file system.
        let pics: Vec<fotema_core::photo::model::Picture> = self.repo.all()?;

        info!("Found {} photos as candidates for cleaning", pics.len());

        let count = pics.par_iter().filter(|p| !p.path.exists()).count();

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoCleanOutput::Completed(count));
            return Ok(());
        }

        if let Err(e) = sender.output(PhotoCleanOutput::Started){
            error!("Failed sending cleanup started: {:?}", e);
        }

        pics.par_iter()
            .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
            .for_each(|pic| {
                if !pic.path.exists() {
                    let mut repo = self.repo.clone();
                    if let Ok(paths) = repo.find_files_to_cleanup(pic.picture_id) {
                        for path in paths {
                            debug!("Deleting {:?}", path);
                            if let Err(e) = std::fs::remove_file(&path) {
                                error!("Failed deleting {:?} with {}", path, e);
                            }
                        }
                    }

                    let result = repo.remove(pic.picture_id);
                    if let Err(e) = result {
                        error!("Failed remove {}: {:?}", pic.picture_id, e);
                    } else {
                        info!("Removed {}", pic.picture_id);
                    }
                }
            });

        info!("Cleaned {} photos in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(PhotoCleanOutput::Completed(count)) {
            error!("Failed sending PhotoCleanOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for PhotoClean {
    type Init = (Arc<AtomicBool>, fotema_core::photo::Repository);
    type Input = PhotoCleanInput;
    type Output = PhotoCleanOutput;

    fn init((stop, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { stop, repo }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoCleanInput::Start => {
                info!("Cleaning photos...");

                if let Err(e) = self.cleanup(&sender) {
                    error!("Failed to clean photos: {}", e);
                }
            }
        };
    }
}
