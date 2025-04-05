// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{error, info};

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};

#[derive(Debug)]
pub enum PhotoExtractMotionInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoExtractMotionOutput {
    // Motion photo extraction has started.
    Started,

    // Motion photo extract has completed
    Completed(usize),
}

pub struct PhotoExtractMotion {
    // Stop flag
    stop: Arc<AtomicBool>,

    extractor: fotema_core::photo::MotionPhotoExtractor,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoExtractMotion {
    fn extract(
        stop: Arc<AtomicBool>,
        repo: fotema_core::photo::Repository,
        extractor: fotema_core::photo::MotionPhotoExtractor,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: ComponentSender<Self>,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let unprocessed: Vec<fotema_core::photo::model::Picture> = repo
            .find_need_motion_photo_extract()?
            .into_iter()
            .filter(|pic| pic.path.exists())
            .collect();

        let count = unprocessed.len();
        info!(
            "Found {} photos as candidates for extracting motion photo videos",
            count
        );

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoExtractMotionOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PhotoExtractMotionOutput::Started);

        progress_monitor.emit(ProgressMonitorInput::Start(TaskName::MotionPhoto, count));

        // One thread per CPU core... makes my laptop sluggish and hot... also likes memory.
        // Might need to consider constraining number of CPUs to use less memory or to
        // keep the computer more response while thumbnail generation is going on.
        unprocessed
            .par_iter()
            .take_any_while(|_| !stop.load(Ordering::Relaxed))
            .for_each(|photo| {
                let result = extractor.extract(&photo.picture_id, &photo.path);

                let result = match result {
                    Ok(opt_video) => repo
                        .clone()
                        .add_motion_photo_video(&photo.picture_id, opt_video),
                    Err(e) => {
                        error!(
                            "Failed extracting motion photo: {:?}: Photo path: {:?}",
                            e, photo.path
                        );
                        repo.clone().mark_broken(&photo.picture_id)
                    }
                };

                if let Err(e) = result {
                    error!(
                        "Failed updating database: {:?}: Photo path: {:?}",
                        e, photo.path
                    );
                }

                progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!(
            "Extracted {} motion photos in {} seconds.",
            count,
            start.elapsed().as_secs()
        );

        progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PhotoExtractMotionOutput::Completed(count));

        Ok(())
    }
}

impl Worker for PhotoExtractMotion {
    type Init = (
        Arc<AtomicBool>,
        fotema_core::photo::MotionPhotoExtractor,
        fotema_core::photo::Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = PhotoExtractMotionInput;
    type Output = PhotoExtractMotionOutput;

    fn init(
        (stop, extractor, repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        PhotoExtractMotion {
            stop,
            extractor,
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoExtractMotionInput::Start => {
                info!("Extracting motion photos...");
                let stop = self.stop.clone();
                let repo = self.repo.clone();
                let extractor = self.extractor.clone();
                let progress_monitor = self.progress_monitor.clone();

                rayon::spawn(move || {
                    if let Err(e) =
                        PhotoExtractMotion::extract(stop, repo, extractor, progress_monitor, sender)
                    {
                        error!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
