// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use fotema_core::Result;
use rayon::prelude::*;


#[derive(Debug)]
pub enum EnrichVideosInput {
    Start,
}

#[derive(Debug)]
pub enum EnrichVideosOutput {
    // Thumbnail generation has started for a given number of videos.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Generated,

    // Thumbnail generation has completed
    Completed,

}

pub struct EnrichVideos {
    enricher: fotema_core::video::Enricher,

    repo: fotema_core::video::Repository,
}

impl EnrichVideos {

    fn enrich(
        repo: fotema_core::video::Repository,
        enricher: fotema_core::video::Enricher,
        sender: &ComponentSender<EnrichVideos>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed_vids: Vec<fotema_core::video::model::Video> = repo
            .all()?
            .into_iter()
            .filter(|vid| !vid.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        let vids_count = unprocessed_vids.len();
        if let Err(e) = sender.output(EnrichVideosOutput::Started(vids_count)){
            println!("Failed sending gen started: {:?}", e);
        }

        unprocessed_vids
            //.par_iter() // don't multiprocess until memory usage is better understood.
            .iter()
            .for_each(|vid| {
                let result = enricher.enrich(&vid.video_id, &vid.path);
                let result = result.and_then(|extra| repo.clone().update(&vid.video_id, &extra));

                if result.is_err() {
                    println!("Failed video add preview for {:?}: {:?}", &vid.path, result);
                }

                if let Err(e) = sender.output(EnrichVideosOutput::Generated) {
                    println!("Failed sending EnrichVideosOutput::Generated: {:?}", e);
                }
            });

        println!("Generated {} video thumbnails in {} seconds.", vids_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(EnrichVideosOutput::Completed) {
            println!("Failed sending EnrichVideosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for EnrichVideos {
    type Init = (fotema_core::video::Enricher, fotema_core::video::Repository);
    type Input = EnrichVideosInput;
    type Output = EnrichVideosOutput;

    fn init((enricher, repo): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = EnrichVideos {
            enricher,
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            EnrichVideosInput::Start => {
                println!("Enriching videos...");
                let repo = self.repo.clone();
                let enricher = self.enricher.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = EnrichVideos::enrich(repo, enricher, &sender) {
                        println!("Failed to enrich videos: {}", e);
                    }
                });
            }
        };
    }
}
