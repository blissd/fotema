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

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
    MediaType
};


#[derive(Debug)]
pub enum ThumbnailPhotosInput {
    Start,
}

#[derive(Debug)]
pub enum ThumbnailPhotosOutput {
    // Thumbnail generation has started.
    Started,

    // Thumbnail generation has completed
    Completed,

}

pub struct ThumbnailPhotos {
    thumbnailer: fotema_core::photo::Thumbnailer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl ThumbnailPhotos {

    fn enrich(
        repo: fotema_core::photo::Repository,
        thumbnailer: fotema_core::photo::Thumbnailer,
        progress_monitor: Arc<Reducer<ProgressMonitor>>,
        sender: ComponentSender<Self>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let _ = sender.output(ThumbnailPhotosOutput::Started);

        let mut unprocessed: Vec<fotema_core::photo::model::Picture> = repo
            .all()?
            .into_iter()
            .filter(|pic| !pic.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        // should be ascending time order from database, so reverse to process newest items first
        unprocessed.reverse();

        let count = unprocessed.len();

        progress_monitor.emit(ProgressMonitorInput::Start(TaskName::Thumbnail(MediaType::Photo), count));

        unprocessed
            //.par_iter() // don't multiprocess until memory usage is better understood.
            .iter()
            .for_each(|pic| {
                let result = block_on(async {thumbnailer.thumbnail(&pic.picture_id, &pic.path).await});

                // TODO is it faster to persist all thumbnail paths to database in a
                // batch at the end instead of one by one?
                let result = result.and_then(|thumbnail_path| repo.clone().add_thumbnail(&pic.picture_id, &thumbnail_path));

                if let Err(e) = result {
                    println!("Failed add_thumbnail: {:?}", e);
                }

                progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        println!("Generated {} photo thumbnails in {} seconds.", count, start.elapsed().as_secs());

        progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(ThumbnailPhotosOutput::Completed);

        Ok(())
    }
}

impl Worker for ThumbnailPhotos {
    type Init = (fotema_core::photo::Thumbnailer, fotema_core::photo::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = ThumbnailPhotosInput;
    type Output = ThumbnailPhotosOutput;

    fn init((thumbnailer, repo, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = ThumbnailPhotos {
            thumbnailer,
            repo,
            progress_monitor,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ThumbnailPhotosInput::Start => {
                println!("Generating photo thumbnails...");
                let repo = self.repo.clone();
                let thumbnailer = self.thumbnailer.clone();
                let progress_monitor = self.progress_monitor.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = ThumbnailPhotos::enrich(repo, thumbnailer, progress_monitor, sender) {
                        println!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
