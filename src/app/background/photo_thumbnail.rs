// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use relm4::Reducer;
use rayon::prelude::*;
use futures::executor::block_on;
use anyhow::*;
use std::sync::Arc;
use tracing::{event, Level};

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
    MediaType
};


#[derive(Debug)]
pub enum PhotoThumbnailInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoThumbnailOutput {
    // Thumbnail generation has started.
    Started,

    // Thumbnail generation has completed
    Completed,

}

pub struct PhotoThumbnail {
    thumbnailer: fotema_core::photo::Thumbnailer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoThumbnail {

    fn enrich(
        repo: fotema_core::photo::Repository,
        thumbnailer: fotema_core::photo::Thumbnailer,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: ComponentSender<Self>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let _ = sender.output(PhotoThumbnailOutput::Started);

        let mut unprocessed: Vec<fotema_core::photo::model::Picture> = repo
            .all()?
            .into_iter()
            .filter(|pic| !pic.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        // should be ascending time order from database, so reverse to process newest items first
        unprocessed.reverse();

        let count = unprocessed.len();

        progress_monitor.emit(ProgressMonitorInput::Start(TaskName::Thumbnail(MediaType::Photo), count));

        // One thread per CPU core... makes my laptop sluggish and hot... also likes memory.
        // Might need to consider constraining number of CPUs to use less memory or to
        // keep the computer more response while thumbnail generation is going on.
        unprocessed
            .par_iter()
            .for_each(|pic| {
                let result = block_on(async {thumbnailer.thumbnail(&pic.picture_id, &pic.path).await})
                    .and_then(|thumbnail_path| repo.clone().add_thumbnail(&pic.picture_id, &thumbnail_path))
                    .with_context(|| format!("Photo path: {:?}", pic.path));

                if let Err(e) = result {
                    event!(Level::ERROR, "Failed add_thumbnail: {:?}", e);
                    let _ = repo.clone().mark_broken(&pic.picture_id);
                }

                progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        event!(Level::INFO, "Generated {} photo thumbnails in {} seconds.", count, start.elapsed().as_secs());

        progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PhotoThumbnailOutput::Completed);

        Ok(())
    }
}

impl Worker for PhotoThumbnail {
    type Init = (fotema_core::photo::Thumbnailer, fotema_core::photo::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = PhotoThumbnailInput;
    type Output = PhotoThumbnailOutput;

    fn init((thumbnailer, repo, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = PhotoThumbnail {
            thumbnailer,
            repo,
            progress_monitor,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoThumbnailInput::Start => {
                event!(Level::INFO, "Generating photo thumbnails...");
                let repo = self.repo.clone();
                let thumbnailer = self.thumbnailer.clone();
                let progress_monitor = self.progress_monitor.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = PhotoThumbnail::enrich(repo, thumbnailer, progress_monitor, sender) {
                        event!(Level::ERROR, "Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
