// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use fotema_core::google_metadata::enrich_photo;
use fotema_core::photo::metadata;
use rayon::prelude::*;
use relm4::Worker;
use relm4::prelude::*;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{error, info};

#[derive(Debug)]
pub enum PhotoEnrichTaskInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoEnrichTaskOutput {
    // Metadata enrichment started.
    Started,

    // Metadata enrichment completed
    Completed(usize),
}

pub struct PhotoEnrichTask {
    // Stop flag
    stop: Arc<AtomicBool>,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl PhotoEnrichTask {
    fn enrich(
        stop: Arc<AtomicBool>,
        mut repo: fotema_core::photo::Repository,
        sender: &ComponentSender<PhotoEnrichTask>,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let unprocessed = repo.find_need_metadata_update()?;

        let count = unprocessed.len();
        info!("Found {} photos as candidates for enriching", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoEnrichTaskOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PhotoEnrichTaskOutput::Started);

        let metadatas = unprocessed
            .par_iter()
            .take_any_while(|_| !stop.load(Ordering::Relaxed))
            .flat_map(|pic| {
                let result = metadata::from_path(&pic.sandbox_path());
                let result = enrich_photo(result.ok(), &pic.sandbox_path());
                result.map(|m| (pic.picture_id, m))
            })
            .collect();

        repo.add_metadatas(metadatas)?;

        info!(
            "Extracted {} photo metadatas in {} seconds.",
            count,
            start.elapsed().as_secs()
        );

        if let Err(e) = sender.output(PhotoEnrichTaskOutput::Completed(count)) {
            error!("Failed sending PhotoEnrichTaskOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for PhotoEnrichTask {
    type Init = (Arc<AtomicBool>, fotema_core::photo::Repository);
    type Input = PhotoEnrichTaskInput;
    type Output = PhotoEnrichTaskOutput;

    fn init((stop, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        PhotoEnrichTask { stop, repo }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoEnrichTaskInput::Start => {
                info!("Enriching photos...");
                let repo = self.repo.clone();
                let stop = self.stop.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = PhotoEnrichTask::enrich(stop, repo, &sender) {
                        error!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
