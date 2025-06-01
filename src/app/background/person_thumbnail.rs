// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use futures::executor::block_on;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{error, info};

use std::panic;

use fotema_core::thumbnailify;
use fotema_core::thumbnailify::ThumbnailSize;
use fotema_core::photo::thumbnailer::PhotoThumbnailer;

use crate::app::components::progress_monitor::{
    MediaType, ProgressMonitor, ProgressMonitorInput, TaskName,
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

    thumbnails_path: PathBuf,
    thumbnailer: fotema_core::photo::PhotoThumbnailer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PersonThumbnailTask {
    fn enrich(
        stop: Arc<AtomicBool>,
        repo: fotema_core::photo::Repository,
        thumbnails_path: &Path,
        thumbnailer: PhotoThumbnailer,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: ComponentSender<Self>,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let mut unprocessed: Vec<fotema_core::photo::model::Picture> = repo
            .all()?
            .into_iter()
            .filter(|pic| pic.path.exists())
            .filter(|pic| {
                let thumb_hash = pic.thumbnail_hash();
                let large_path = thumbnailify::get_thumbnail_hash_output(
                    thumbnails_path,
                    &thumb_hash,
                    ThumbnailSize::XLarge,
                );
                !large_path.exists()
            })
            .collect();

        // should be ascending time order from database, so reverse to process newest items first
        unprocessed.reverse();

        let count = unprocessed.len();
        info!("Found {} photos to generate thumbnails for", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PersonThumbnailTaskOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PersonThumbnailTaskOutput::Started);

        progress_monitor.emit(ProgressMonitorInput::Start(
            TaskName::Thumbnail(MediaType::Photo),
            count,
        ));

        // One thread per CPU core... makes my laptop sluggish and hot... also likes memory.
        // Might need to consider constraining number of CPUs to use less memory or to
        // keep the computer more response while thumbnail generation is going on.
        unprocessed
            .par_iter()
            .take_any_while(|_| !stop.load(Ordering::Relaxed))
            .for_each(|pic| {
                // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
                // an error but doesn't panic.
                let result = panic::catch_unwind(|| {
                    block_on(async { thumbnailer.thumbnail(&pic.path).await })
                });

                // If we got an err, then there was a panic.
                // If we got Ok(Err(e)) there wasn't a panic, but we still failed.
                if let Ok(Err(e)) = result {
                    error!(
                        "Failed generate or add thumbnail: {:?}: Photo path: {:?}",
                        e.root_cause(),
                        pic.path
                    );
                    let _ = repo.clone().mark_broken(&pic.picture_id);
                } else if result.is_err() {
                    error!(
                        "Panicked generate or add thumbnail: Photo path: {:?}",
                        pic.path
                    );
                    let _ = repo.clone().mark_broken(&pic.picture_id);
                }

                progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!(
            "Generated {} photo thumbnails in {} seconds.",
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
        PathBuf,
        PhotoThumbnailer,
        fotema_core::photo::Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = PersonThumbnailTaskInput;
    type Output = PersonThumbnailTaskOutput;

    fn init(
        (stop, thumbnails_path, thumbnailer, repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        PersonThumbnailTask {
            stop,
            thumbnails_path: thumbnails_path.into(),
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
                let thumbnails_path = self.thumbnails_path.clone();
                let thumbnailer = self.thumbnailer.clone();
                let progress_monitor = self.progress_monitor.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = PersonThumbnailTask::enrich(
                        stop,
                        repo,
                        &thumbnails_path,
                        thumbnailer,
                        progress_monitor,
                        sender,
                    ) {
                        error!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
