// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use rayon::prelude::*;
use futures::executor::block_on;
use anyhow::*;


#[derive(Debug)]
pub enum ThumbnailPhotosInput {
    Start,
}

#[derive(Debug)]
pub enum ThumbnailPhotosOutput {
    // Thumbnail generation has started for a given number of images.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Generated,

    // Thumbnail generation has completed
    Completed,

}

pub struct ThumbnailPhotos {
    thumbnailer: fotema_core::photo::Thumbnailer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl ThumbnailPhotos {

    fn enrich(
        repo: fotema_core::photo::Repository,
        thumbnailer: fotema_core::photo::Thumbnailer,
        sender: &ComponentSender<ThumbnailPhotos>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let mut unprocessed: Vec<fotema_core::photo::model::Picture> = repo
            .all()?
            .into_iter()
            .filter(|pic| !pic.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        // should be ascending time order from database, so reverse to process newest items first
        unprocessed.reverse();

        let count = unprocessed.len();
        if let Err(e) = sender.output(ThumbnailPhotosOutput::Started(count)){
            println!("Failed sending gen started: {:?}", e);
        }

        unprocessed
            .par_iter() // don't multiprocess until memory usage is better understood.
            //.iter()
            .for_each(|pic| {
                let result = block_on(async {thumbnailer.thumbnail(&pic.picture_id, &pic.path).await});
                let result = result.and_then(|thumbnail_path| repo.clone().add_thumbnail(&pic.picture_id, &thumbnail_path));

                if let Err(e) = result {
                    println!("Failed add_thumbnail: {:?}", e);
                } else if let Err(e) = sender.output(ThumbnailPhotosOutput::Generated) {
                    println!("Failed sending ThumbnailPhotosOutput::Generated: {:?}", e);
                }
            });

        println!("Generated {} photo thumbnails in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(ThumbnailPhotosOutput::Completed) {
            println!("Failed sending ThumbnailPhotosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for ThumbnailPhotos {
    type Init = (fotema_core::photo::Thumbnailer, fotema_core::photo::Repository);
    type Input = ThumbnailPhotosInput;
    type Output = ThumbnailPhotosOutput;

    fn init((thumbnailer, repo): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = ThumbnailPhotos {
            thumbnailer,
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ThumbnailPhotosInput::Start => {
                println!("Generating photo thumbnails...");
                let repo = self.repo.clone();
                let thumbnailer = self.thumbnailer.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = ThumbnailPhotos::enrich(repo, thumbnailer, &sender) {
                        println!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
