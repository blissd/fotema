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
pub enum PhotoEnrichInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoEnrichOutput {
    // Metadata enrichment started.
    Started,

    // Metadata enrichment completed
    Completed,

}

pub struct PhotoEnrich {
    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl PhotoEnrich {

    fn enrich(
        mut repo: fotema_core::photo::Repository,
        sender: &ComponentSender<PhotoEnrich>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed = repo.find_need_metadata_update()?;

        let count = unprocessed.len();

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoEnrichOutput::Completed);
            return Ok(());
        }

        let _ = sender.output(PhotoEnrichOutput::Started);

        let metadatas = unprocessed
            .par_iter()
            .flat_map(|pic| {
                let result = metadata::from_path(&pic.path);
                result.map(|m| (pic.picture_id, m))
            })
            .collect();

        repo.add_metadatas(metadatas)?;

        event!(Level::INFO, "Extracted {} photo metadatas in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(PhotoEnrichOutput::Completed) {
            event!(Level::ERROR, "Failed sending PhotoEnrichOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for PhotoEnrich {
    type Init = fotema_core::photo::Repository;
    type Input = PhotoEnrichInput;
    type Output = PhotoEnrichOutput;

    fn init(repo: Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = PhotoEnrich {
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoEnrichInput::Start => {
                event!(Level::INFO, "Enriching photos...");
                let repo = self.repo.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = PhotoEnrich::enrich(repo, &sender) {
                        event!(Level::ERROR, "Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
