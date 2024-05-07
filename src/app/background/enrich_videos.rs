// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use anyhow::*;
use fotema_core::video::metadata;

#[derive(Debug)]
pub enum EnrichVideosInput {
    Start,
}

#[derive(Debug)]
pub enum EnrichVideosOutput {
    // Thumbnail generation has started.
    Started,

    // Thumbnail generation has completed
    Completed,
}

pub struct EnrichVideos {
    repo: fotema_core::video::Repository,
}

impl EnrichVideos {

    fn enrich(
        mut repo: fotema_core::video::Repository,
        sender: &ComponentSender<EnrichVideos>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let _ = sender.output(EnrichVideosOutput::Started);

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

        println!("Enriched {} videos in {} seconds.", count, start.elapsed().as_secs());

        if let Err(e) = sender.output(EnrichVideosOutput::Completed) {
            println!("Failed sending EnrichVideosOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for EnrichVideos {
    type Init = fotema_core::video::Repository;
    type Input = EnrichVideosInput;
    type Output = EnrichVideosOutput;

    fn init(repo: Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = EnrichVideos {
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            EnrichVideosInput::Start => {
                println!("Enriching videos...");
                let repo = self.repo.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = EnrichVideos::enrich(repo, &sender) {
                        println!("Failed to enrich videos: {}", e);
                    }
                });
            }
        };
    }
}
