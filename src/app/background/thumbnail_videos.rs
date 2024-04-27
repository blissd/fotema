// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use anyhow::*;


#[derive(Debug)]
pub enum ThumbnailVideosInput {
    Start,
}

#[derive(Debug)]
pub enum ThumbnailVideosOutput {
    // Thumbnail generation has started for a given number of videos.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Generated,

    // Thumbnail generation has completed
    Completed,

}

pub struct ThumbnailVideos {
    thumbnailer: fotema_core::video::Thumbnailer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::video::Repository,
}

impl ThumbnailVideos {

    fn enrich(
        repo: fotema_core::video::Repository,
        thumbnailer: fotema_core::video::Thumbnailer,
        sender: &ComponentSender<ThumbnailVideos>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed_vids: Vec<fotema_core::video::model::Video> = repo
            .all()?
            .into_iter()
            .filter(|vid| !vid.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        let vid_count = unprocessed_vids.len();
        if let Err(e) = sender.output(ThumbnailVideosOutput::Started(vid_count)){
            println!("Failed sending gen started: {:?}", e);
        }

        unprocessed_vids
            //.par_iter() // don't multiprocess until memory usage is better understood.
            .iter()
            .for_each(|vid| {
                let result = thumbnailer.thumbnail(&vid.video_id, &vid.path);
                let result = result.and_then(|thumbnail_path| repo.clone().add_thumbnail(&vid.video_id, &thumbnail_path));

                if let Err(e) = result {
                    println!("Failed add_thumbnail: {:?}", e);
                } else if let Err(e) = sender.output(ThumbnailVideosOutput::Generated) {
                    println!("Failed sending ThumbnailVideosOutput::Generated: {:?}", e);
                }
            });

        println!("Generated {} video thumbnails in {} seconds.", vid_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(ThumbnailVideosOutput::Completed) {
            println!("Failed sending ThumbnailVideosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for ThumbnailVideos {
    type Init = (fotema_core::video::Thumbnailer, fotema_core::video::Repository);
    type Input = ThumbnailVideosInput;
    type Output = ThumbnailVideosOutput;

    fn init((thumbnailer, repo): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = Self {
            thumbnailer,
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ThumbnailVideosInput::Start => {
                println!("Generating photo thumbnails...");
                let repo = self.repo.clone();
                let thumbnailer = self.thumbnailer.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = ThumbnailVideos::enrich(repo, thumbnailer, &sender) {
                        println!("Failed to update video thumbnails: {}", e);
                    }
                });
            }
        };
    }
}
