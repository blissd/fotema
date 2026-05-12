// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;
use std::panic;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use gdt_cpus;
use tracing::{error, info};

use fotema_core::thumbnailify;
use fotema_core::thumbnailify::ThumbnailSize;
use fotema_core::video::{Repository, Video, VideoThumbnailer};

use crate::app::components::progress_monitor::{
    ProgressMonitor, ProgressMonitorInput, TaskName, ThumbnailType,
};

#[derive(Debug)]
pub enum VideoThumbnailTaskInput {
    Start,
}

#[derive(Debug)]
pub enum VideoThumbnailTaskOutput {
    // Thumbnail generation has started
    Started,

    // Thumbnail generation has completed
    Completed(usize),
}

pub struct VideoThumbnailTask {
    // Stop flag
    stop: Arc<AtomicBool>,

    // Background thumbnail generation enabled?
    enabled: Arc<AtomicBool>,

    thumbnails_path: PathBuf,
    thumbnailer: VideoThumbnailer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl VideoThumbnailTask {
    fn enrich(
        stop: Arc<AtomicBool>,
        enabled: Arc<AtomicBool>,
        repo: Repository,
        thumbnails_path: &Path,
        thumbnailer: VideoThumbnailer,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: ComponentSender<VideoThumbnailTask>,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let mut unprocessed: Vec<Video> = repo
            .all()?
            .into_iter()
            .filter(|vid| vid.path.exists())
            .filter(|vid| {
                let thumb_hash = vid.thumbnail_hash();
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
        info!("Found {} videos to generate thumbnails for", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(VideoThumbnailTaskOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(VideoThumbnailTaskOutput::Started);

        progress_monitor.emit(ProgressMonitorInput::Start(
            TaskName::Thumbnail(ThumbnailType::Video),
            count,
        ));

        unprocessed
            //.par_iter()
            //.take_any_while(|_| !stop.load(Ordering::Relaxed))
            .into_iter()
            .take_while(|_| !stop.load(Ordering::Relaxed))
            .take_while(|_| enabled.load(Ordering::Relaxed))
            .for_each(|vid| {
                // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
                // an error but doesn't panic.
                let result = panic::catch_unwind(|| thumbnailer.thumbnail(&vid.path));

                // If we got an err, then there was a panic.
                // If we got Ok(Err(e)) there wasn't a panic, but we still failed.
                if let Ok(Err(e)) = result {
                    error!(
                        "Failed generate or add thumbnail: {:?}: Video path: {:?}",
                        e.root_cause(),
                        vid.path
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

        let _ = sender.output(VideoThumbnailTaskOutput::Completed(count));

        Ok(())
    }
}

impl Worker for VideoThumbnailTask {
    type Init = (
        Arc<AtomicBool>,
        Arc<AtomicBool>,
        PathBuf,
        VideoThumbnailer,
        Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = VideoThumbnailTaskInput;
    type Output = VideoThumbnailTaskOutput;

    fn init(
        (stop, enabled, thumbnails_path, thumbnailer, repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        Self {
            stop,
            enabled,
            thumbnails_path: thumbnails_path.into(),
            thumbnailer,
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoThumbnailTaskInput::Start => {
                info!("Generating video thumbnails...");
                let stop = self.stop.clone();
                let enabled = self.enabled.clone();
                let repo = self.repo.clone();
                let thumbnails_path = self.thumbnails_path.clone();
                let thumbnailer = self.thumbnailer.clone();
                let progress_monitor = self.progress_monitor.clone();

                // Avoid runtime panic from calling block_on
                thread::spawn(move || {
                    if let Err(err) =
                        gdt_cpus::set_thread_priority(gdt_cpus::ThreadPriority::Background)
                    {
                        error!("Failed to lower thread priority: {:?}", err);
                    }
                    if let Err(e) = VideoThumbnailTask::enrich(
                        stop,
                        enabled,
                        repo,
                        &thumbnails_path,
                        thumbnailer,
                        progress_monitor,
                        sender,
                    ) {
                        error!("Failed to update video thumbnails: {}", e);
                    }
                });
            }
        };
    }
}
