// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use anyhow::*;
use fotema_core::video::metadata;

use tracing::{event, Level};

#[derive(Debug)]
pub enum VideoEnrichInput {
    Start,
}

#[derive(Debug)]
pub enum VideoEnrichOutput {
    // Thumbnail generation has started.
    Started,

    // Thumbnail generation has completed
    Completed,
}

pub struct VideoEnrich {
    repo: fotema_core::video::Repository,
}

impl VideoEnrich {

    fn enrich(
        mut repo: fotema_core::video::Repository,
        sender: &ComponentSender<VideoEnrich>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let _ = sender.output(VideoEnrichOutput::Started);

        let unprocessed = repo.find_need_metadata_update()?;

        let count = unprocessed.len();

        let metadatas = unprocessed
            //.par_iter() // don't multiprocess until memory usage is better understood.
            .iter()
            .flat_map(|vid| {
                let result = metadata::from_path(&vid.path);
                result.map(|m| (vid.video_id, m))
            })
            .collect();

        repo.add_metadata(metadatas)?;

        event!(Level::INFO, "Enriched {} videos in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(VideoEnrichOutput::Completed) {
            event!(Level::ERROR, "Failed sending VideoEnrichOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for VideoEnrich {
    type Init = fotema_core::video::Repository;
    type Input = VideoEnrichInput;
    type Output = VideoEnrichOutput;

    fn init(repo: Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = VideoEnrich {
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoEnrichInput::Start => {
                event!(Level::INFO, "Enriching videos...");
                let repo = self.repo.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = VideoEnrich::enrich(repo, &sender) {
                        event!(Level::ERROR, "Failed to enrich videos: {}", e);
                    }
                });
            }
        };
    }
}
