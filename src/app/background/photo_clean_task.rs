// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use rayon::prelude::*;
use relm4::Worker;
use relm4::prelude::*;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{debug, error, info};

#[derive(Debug)]
pub enum PhotoCleanTaskInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoCleanTaskOutput {
    // Thumbnail generation has started for a given number of images.
    Started,

    // Thumbnail generation has completed
    Completed(usize),
}

pub struct PhotoCleanTask {
    // Stop flag
    stop: Arc<AtomicBool>,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl PhotoCleanTask {
    fn cleanup(&mut self, sender: &ComponentSender<Self>) -> Result<()> {
        let start = std::time::Instant::now();

        // Scrub pics from database if they no longer exist on the file system.
        let pics: Vec<fotema_core::photo::model::Picture> = self.repo.all()?;

        info!("Found {} photos as candidates for cleaning", pics.len());

        let count = pics.par_iter().filter(|p| !p.path.exists()).count();

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoCleanTaskOutput::Completed(count));
            return Ok(());
        }

        if let Err(e) = sender.output(PhotoCleanTaskOutput::Started) {
            error!("Failed sending cleanup started: {:?}", e);
        }

        pics.par_iter()
            .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
            .for_each(|pic| {
                if !pic.path.exists() {
                    let mut repo = self.repo.clone();
                    if let Ok(paths) = repo.find_files_to_cleanup(pic.picture_id) {
                        for path in paths {
                            if !path.exists() {
                                continue;
                            }
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

        info!(
            "Cleaned {} photos in {} seconds.",
            count,
            start.elapsed().as_secs()
        );

        if let Err(e) = sender.output(PhotoCleanTaskOutput::Completed(count)) {
            error!("Failed sending PhotoCleanTaskOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for PhotoCleanTask {
    type Init = (Arc<AtomicBool>, fotema_core::photo::Repository);
    type Input = PhotoCleanTaskInput;
    type Output = PhotoCleanTaskOutput;

    fn init((stop, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { stop, repo }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoCleanTaskInput::Start => {
                info!("Cleaning photos...");

                if let Err(e) = self.cleanup(&sender) {
                    error!("Failed to clean photos: {}", e);
                }
            }
        };
    }
}
