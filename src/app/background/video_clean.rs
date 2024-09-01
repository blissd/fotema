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
pub enum VideoCleanInput {
    Start,
}

#[derive(Debug)]
pub enum VideoCleanOutput {
    // Thumbnail generation has started for a given number of images.
    Started,

    // Thumbnail generation has completed
    Completed(usize),

}

pub struct VideoClean {
    // Stop flag
    stop: Arc<AtomicBool>,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::video::Repository,
}

impl VideoClean {

    fn cleanup(&mut self, sender: &ComponentSender<Self>) -> Result<()> {

        let start = std::time::Instant::now();

        // Scrub vids from database if they no longer exist on the file system.
        let vids: Vec<fotema_core::video::model::Video> = self.repo.all()?;

        info!("Found {} videos as candidates for cleaning", vids.len());

        let count = vids.par_iter().filter(|v| !v.path.exists()).count();

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(VideoCleanOutput::Completed(count));
            return Ok(());
        }

        if let Err(e) = sender.output(VideoCleanOutput::Started){
            error!("Failed sending cleanup started: {:?}", e);
        }

        vids.par_iter()
            .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
            .for_each(|vid| {
                if !vid.path.exists() {
                    let mut repo = self.repo.clone();
                    if let Ok(paths) = repo.find_files_to_cleanup(vid.video_id) {
                        for path in paths {
                            debug!("Deleting {:?}", path);
                            if let Err(e) = std::fs::remove_file(&path) {
                                error!("Failed deleting {:?} with {}", path, e);
                            }
                        }
                    }

                    let result = repo.remove(vid.video_id);
                    if let Err(e) = result {
                        error!("Failed remove {}: {:?}", vid.video_id, e);
                    } else {
                        info!("Removed {}", vid.video_id);
                    }
                }
            });

        info!("Cleaned {} videos in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(VideoCleanOutput::Completed(count)) {
            error!("Failed sending VideoCleanOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for VideoClean {
    type Init = (Arc<AtomicBool>, fotema_core::video::Repository);
    type Input = VideoCleanInput;
    type Output = VideoCleanOutput;

    fn init((stop, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { stop, repo }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoCleanInput::Start => {
                info!("Cleaning videos...");

                if let Err(e) = self.cleanup(&sender) {
                    error!("Failed to clean videos: {}", e);
                }
            }
        };
    }
}
