// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use rayon::prelude::*;
use anyhow::*;
use fotema_core::photo::metadata;

use tracing::{event, Level};

#[derive(Debug)]
pub enum EnrichPhotosInput {
    Start,
}

#[derive(Debug)]
pub enum EnrichPhotosOutput {
    // Metadata enrichment started.
    Started,

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

        let _ = sender.output(EnrichPhotosOutput::Started);

        let unprocessed = repo.find_need_metadata_update()?;

        let count = unprocessed.len();

        let metadatas = unprocessed
            .par_iter() // don't multiprocess until memory usage is better understood.
            //.iter()
            .flat_map(|pic| {
                let result = metadata::from_path(&pic.path);
                result.map(|m| (pic.picture_id, m))
            })
            .collect();

        repo.add_metadatas(metadatas)?;

        event!(Level::INFO, "Extracted {} photo metadatas in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(EnrichPhotosOutput::Completed) {
            event!(Level::ERROR, "Failed sending EnrichPhotosOutput::Completed: {:?}", e);
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
                event!(Level::INFO, "Enriching photos...");
                let repo = self.repo.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = EnrichPhotos::enrich(repo, &sender) {
                        event!(Level::ERROR, "Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
