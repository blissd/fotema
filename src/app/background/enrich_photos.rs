// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use fotema_core::Result;
use rayon::prelude::*;
use futures::executor::block_on;


#[derive(Debug)]
pub enum EnrichPhotosInput {
    Start,
}

#[derive(Debug)]
pub enum EnrichPhotosOutput {
    // Thumbnail generation has started for a given number of images.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Generated,

    // Thumbnail generation has completed
    Completed,

}

pub struct EnrichPhotos {
    enricher: fotema_core::photo::Enricher,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl EnrichPhotos {

    fn enrich(
        repo: fotema_core::photo::Repository,
        enricher: fotema_core::photo::Enricher,
        sender: &ComponentSender<EnrichPhotos>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed_pics: Vec<fotema_core::photo::model::Picture> = repo
            .all()?
            .into_iter()
            .filter(|pic| !pic.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        let pics_count = unprocessed_pics.len();
        if let Err(e) = sender.output(EnrichPhotosOutput::Started(pics_count)){
            println!("Failed sending gen started: {:?}", e);
        }

        unprocessed_pics
            //.par_iter() // don't multiprocess until memory usage is better understood.
            .iter()
            .for_each(|pic| {
                let result = block_on(async {enricher.enrich(&pic.picture_id, &pic.path).await});
                let result = result.and_then(|extra| repo.clone().update(&pic.picture_id, &extra));

                if let Err(e) = result {
                    println!("Failed add_preview: {:?}", e);
                } else if let Err(e) = sender.output(EnrichPhotosOutput::Generated) {
                    println!("Failed sending EnrichPhotosOutput::Generated: {:?}", e);
                }
            });

        println!("Generated {} previews in {} seconds.", pics_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(EnrichPhotosOutput::Completed) {
            println!("Failed sending EnrichPhotosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for EnrichPhotos {
    type Init = (fotema_core::photo::Enricher, fotema_core::photo::Repository);
    type Input = EnrichPhotosInput;
    type Output = EnrichPhotosOutput;

    fn init((enricher, repo): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = EnrichPhotos {
            enricher,
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            EnrichPhotosInput::Start => {
                println!("Enriching photos...");
                let repo = self.repo.clone();
                let previewer = self.enricher.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = EnrichPhotos::enrich(repo, previewer, &sender) {
                        println!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
