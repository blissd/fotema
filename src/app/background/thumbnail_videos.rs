// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use std::sync::{Arc, Mutex};
use photos_core::Result;
use rayon::prelude::*;


#[derive(Debug)]
pub enum VideoThumbnailsInput {
    Start,
}

#[derive(Debug)]
pub enum VideoThumbnailsOutput {
    // Thumbnail generation has started for a given number of videos.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Generated,

    // Thumbnail generation has completed
    Completed,

}

pub struct VideoThumbnails {
    thumbnailer: photos_core::video::Thumbnailer,

    repo: Arc<Mutex<photos_core::video::Repository>>,
}

impl VideoThumbnails {

    fn update_thumbnails(
        repo: Arc<Mutex<photos_core::video::Repository>>,
        thumbnailer: photos_core::video::Thumbnailer,
        sender: &ComponentSender<VideoThumbnails>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed_vids: Vec<photos_core::video::repo::Video> = repo
            .lock()
            .unwrap()
            .all()?
            .into_iter()
            .filter(|vid| !vid.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        let vids_count = unprocessed_vids.len();
        if let Err(e) = sender.output(VideoThumbnailsOutput::Started(vids_count)){
            println!("Failed sending gen started: {:?}", e);
        }

        unprocessed_vids.par_iter()
            .flat_map(|vid| {
                let result = thumbnailer.set_thumbnail(vid.clone());
                if let Err(ref e) = result {
                    println!("Failed setting video preview: {:?}", e);
                }
                result
            })
            .for_each(|vid| {
                let result = repo.lock().unwrap().update(&vid);
                if let Err(e) = result {
                    println!("Failed video add preview: {:?}", e);
                } else if let Err(e) = sender.output(VideoThumbnailsOutput::Generated) {
                    println!("Failed sending VideoPreviewsOutput::Generated: {:?}", e);
                }
            });

        println!("Generated {} video thumbnails in {} seconds.", vids_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(VideoThumbnailsOutput::Completed) {
            println!("Failed sending VideoThumbnailsOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for VideoThumbnails {
    type Init = (photos_core::video::Thumbnailer, Arc<Mutex<photos_core::video::Repository>>);
    type Input = VideoThumbnailsInput;
    type Output = VideoThumbnailsOutput;

    fn init((thumbnailer, repo): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = VideoThumbnails {
            thumbnailer,
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoThumbnailsInput::Start => {
                println!("Generating video thumbnails...");
                let repo = self.repo.clone();
                let thumbnailer = self.thumbnailer.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = VideoThumbnails::update_thumbnails(repo, thumbnailer, &sender) {
                        println!("Failed to update video thumbnails: {}", e);
                    }
                });
            }
        };
    }
}
