// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use relm4::shared_state::Reducer;
use anyhow::*;
use fotema_core::video::metadata;
use rayon::prelude::*;

use tracing::{error, info};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
    MediaType
};

#[derive(Debug)]
pub enum VideoEnrichInput {
    Start,
}

#[derive(Debug)]
pub enum VideoEnrichOutput {
    // Thumbnail generation has started.
    Started,

    // Thumbnail generation has completed
    Completed(usize),
}

pub struct VideoEnrich {
    // Stop flag
    stop: Arc<AtomicBool>,
    repo: fotema_core::video::Repository,
    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl VideoEnrich {

    fn enrich(
        stop: Arc<AtomicBool>,
        mut repo: fotema_core::video::Repository,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: &ComponentSender<VideoEnrich>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed = repo.find_need_metadata_update()?;

        let count = unprocessed.len();
         info!("Found {} videos as candidates for enriching", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(VideoEnrichOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(VideoEnrichOutput::Started);

        progress_monitor.emit(ProgressMonitorInput::Start(TaskName::Enrich(MediaType::Video), count));

        let metadatas = unprocessed
            .par_iter()
            .take_any_while(|_| !stop.load(Ordering::Relaxed))
            .flat_map(|vid| {
                let result = metadata::from_path(&vid.path);
                progress_monitor.emit(ProgressMonitorInput::Advance);
                result.map(|m| (vid.video_id, m))
            })
            .collect();

        repo.add_metadata(metadatas)?;

        progress_monitor.emit(ProgressMonitorInput::Complete);

        info!("Enriched {} videos in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(VideoEnrichOutput::Completed(count)) {
            error!("Failed sending VideoEnrichOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for VideoEnrich {
    type Init = (Arc<AtomicBool>, fotema_core::video::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = VideoEnrichInput;
    type Output = VideoEnrichOutput;

    fn init((stop, repo, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        VideoEnrich {
            stop,
            repo,
            progress_monitor,
        }
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoEnrichInput::Start => {
                info!("Enriching videos...");
                let stop = self.stop.clone();
                let repo = self.repo.clone();
                let progress_monitor = self.progress_monitor.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = VideoEnrich::enrich(stop, repo, progress_monitor, &sender) {
                        error!("Failed to enrich videos: {}", e);
                    }
                });
            }
        };
    }
}
