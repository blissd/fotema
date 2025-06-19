// SPDX-FileCopyrightText: © 2024-2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use futures::executor::block_on;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{error, info};
use fotema_core::FlatpakPathBuf;
use fotema_core::people::model::DetectedFace;

use fotema_core::people::PersonThumbnailer;


use crate::app::components::progress_monitor::{
    ThumbnailType, ProgressMonitor, ProgressMonitorInput, TaskName,
};

#[derive(Debug)]
pub enum PersonThumbnailTaskInput {
    Start,
}

#[derive(Debug)]
pub enum PersonThumbnailTaskOutput {
    // Thumbnail generation has started.
    Started,

    // Thumbnail generation has completed
    Completed(usize),
}

pub struct PersonThumbnailTask {
    // Stop flag
    stop: Arc<AtomicBool>,

    thumbnailer: fotema_core::people::PersonThumbnailer,

    // FIXME use people repo
    repo: fotema_core::photo::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PersonThumbnailTask {
    fn enrich(
        stop: Arc<AtomicBool>,
        repo: fotema_core::photo::Repository,
        thumbnailer: PersonThumbnailer,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: ComponentSender<Self>,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let unprocessed: Vec<(FlatpakPathBuf, DetectedFace)> = repo
            .find_people_for_thumbnails()?
            .into_iter()
            .filter(|(path, _face)| path.exists())
            /*.filter(|(path, face)| {
                let thumb_hash = path.thumbnail_hash();
                let large_path = thumbnailify::get_thumbnail_hash_output(
                    thumbnails_path,
                    &thumb_hash,
                    ThumbnailSize::XLarge,
                );
                !large_path.exists()
            })*/
            .collect();

        let count = unprocessed.len();
        info!("Found {} people to generate thumbnails for", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PersonThumbnailTaskOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PersonThumbnailTaskOutput::Started);

        progress_monitor.emit(ProgressMonitorInput::Start(
            TaskName::Thumbnail(ThumbnailType::Face),
            count,
        ));

        unprocessed
            .par_iter()
            .take_any_while(|_| !stop.load(Ordering::Relaxed))
            .for_each(|(path, face)| {
                let result = block_on(async { thumbnailer.thumbnail(path, face).await });

                // If we got an err, then there was a panic.
                // If we got Ok(Err(e)) there wasn't a panic, but we still failed.
                if let Err(e) = result {
                    error!(
                        "Failed generate or add person thumbnail: {:?}: Photo path: {:?}",
                        e.root_cause(),
                        path
                    );
                }
                progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!(
            "Generated {} person thumbnails in {} seconds.",
            count,
            start.elapsed().as_secs()
        );

        progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PersonThumbnailTaskOutput::Completed(count));

        Ok(())
    }
}

impl Worker for PersonThumbnailTask {
    type Init = (
        Arc<AtomicBool>,
        PersonThumbnailer,
        fotema_core::photo::Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = PersonThumbnailTaskInput;
    type Output = PersonThumbnailTaskOutput;

    fn init(
        (stop, thumbnailer, repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        PersonThumbnailTask {
            stop,
            thumbnailer,
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PersonThumbnailTaskInput::Start => {
                info!("Generating person thumbnails...");
                let stop = self.stop.clone();
                let repo = self.repo.clone();
                let thumbnailer = self.thumbnailer.clone();
                let progress_monitor = self.progress_monitor.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = PersonThumbnailTask::enrich(
                        stop,
                        repo,
                        thumbnailer,
                        progress_monitor,
                        sender,
                    ) {
                        error!("Failed to update people thumbnails: {}", e);
                    }
                });
            }
        };
    }
}
