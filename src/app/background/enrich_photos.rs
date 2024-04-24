// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use rayon::prelude::*;
use anyhow::*;
use fotema_core::photo::metadata;


#[derive(Debug)]
pub enum EnrichPhotosInput {
    Start,
}

#[derive(Debug)]
pub enum EnrichPhotosOutput {
    // Metadata enrichment started.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Enriched,

    // Metadata enrichment completed
    Completed,

}

pub struct EnrichPhotos {
    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl EnrichPhotos {

    fn enrich(
        mut repo: fotema_core::photo::Repository,
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

        let metadatas = unprocessed_pics
            .par_iter() // don't multiprocess until memory usage is better understood.
            //.iter()
            .flat_map(|pic| {
                let result = metadata::from_path(&pic.path);
                result.map(|m| (pic.picture_id, m))
            })
            .collect();

        let result = repo.add_metadatas(metadatas);

        println!("Extracted {} photo metadatas in {} seconds.", pics_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(EnrichPhotosOutput::Completed) {
            println!("Failed sending EnrichPhotosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for EnrichPhotos {
    type Init = fotema_core::photo::Repository;
    type Input = EnrichPhotosInput;
    type Output = EnrichPhotosOutput;

    fn init(repo: Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = EnrichPhotos {
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            EnrichPhotosInput::Start => {
                println!("Enriching photos...");
                let repo = self.repo.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = EnrichPhotos::enrich(repo, &sender) {
                        println!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
