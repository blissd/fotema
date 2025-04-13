// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;
use std::panic;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{error, info};

use fotema_core::video::{Repository, Thumbnailer, Video};

use crate::app::components::progress_monitor::{
    MediaType, ProgressMonitor, ProgressMonitorInput, TaskName,
};

#[derive(Debug)]
pub enum VideoThumbnailInput {
    Start,
}

#[derive(Debug)]
pub enum VideoThumbnailOutput {
    // Thumbnail generation has started
    Started,

    // Thumbnail generation has completed
    Completed(usize),
}

pub struct VideoThumbnail {
    // Stop flag
    stop: Arc<AtomicBool>,

    thumbnailer: Thumbnailer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl VideoThumbnail {
    fn enrich(
        stop: Arc<AtomicBool>,
        repo: Repository,
        thumbnailer: Thumbnailer,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: ComponentSender<VideoThumbnail>,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let mut unprocessed: Vec<Video> = repo
            .all()?
            .into_iter()
            .filter(|vid| vid.path.exists())
            .filter(|vid| !vid.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        // should be ascending time order from database, so reverse to process newest items first
        unprocessed.reverse();

        let count = unprocessed.len();
        info!("Found {} videos to generate thumbnails for", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(VideoThumbnailOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(VideoThumbnailOutput::Started);

        progress_monitor.emit(ProgressMonitorInput::Start(
            TaskName::Thumbnail(MediaType::Video),
            count,
        ));

        unprocessed
            .par_iter()
            .take_any_while(|_| !stop.load(Ordering::Relaxed))
            .for_each(|vid| {
                // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
                // an error but doesn't panic.
                let result = panic::catch_unwind(|| {
                    thumbnailer
                        .thumbnail(&vid.host_path, &vid.path)
                });

                // If we got an err, then there was a panic.
                // If we got Ok(Err(e)) there wasn't a panic, but we still failed.
                if let Ok(Err(e)) = result {
                    error!(
                        "Failed generate or add thumbnail: {:?}: Video path: {:?}",
                        e, vid.path
                    );
                    let _ = repo.clone().mark_broken(&vid.video_id);
                } else if result.is_err() {
                    error!(
                        "Panicked generate or add thumbnail: Video path: {:?}",
                        vid.path
                    );
                    let _ = repo.clone().mark_broken(&vid.video_id);
                }

                progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!(
            "Generated {} video thumbnails in {} seconds.",
            count,
            start.elapsed().as_secs()
        );

        progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(VideoThumbnailOutput::Completed(count));

        Ok(())
    }
}

impl Worker for VideoThumbnail {
    type Init = (
        Arc<AtomicBool>,
        Thumbnailer,
        Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = VideoThumbnailInput;
    type Output = VideoThumbnailOutput;

    fn init(
        (stop, thumbnailer, repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        Self {
            stop,
            thumbnailer,
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoThumbnailInput::Start => {
                info!("Generating video thumbnails...");
                let stop = self.stop.clone();
                let repo = self.repo.clone();
                let thumbnailer = self.thumbnailer.clone();
                let progress_monitor = self.progress_monitor.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) =
                        VideoThumbnail::enrich(stop, repo, thumbnailer, progress_monitor, sender)
                    {
                        error!("Failed to update video thumbnails: {}", e);
                    }
                });
            }
        };
    }
}
