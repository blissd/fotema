// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use fotema_core::Result;
use rayon::prelude::*;
use futures::executor::block_on;


#[derive(Debug)]
pub enum GeneratePreviewsInput {
    Start,
}

#[derive(Debug)]
pub enum GeneratePreviewsOutput {
    // Thumbnail generation has started for a given number of images.
    Started(usize),

    // Thumbnail has been generated for a photo.
    Generated,

    // Thumbnail generation has completed
    Completed,

}

pub struct GeneratePreviews {
    previewer: fotema_core::photo::Previewer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,
}

impl GeneratePreviews {

    fn update_previews(
        repo: fotema_core::photo::Repository,
        previewer: fotema_core::photo::Previewer,
        sender: &ComponentSender<GeneratePreviews>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed_pics: Vec<fotema_core::photo::model::Picture> = repo
            .all()?
            .into_iter()
            .filter(|pic| !pic.thumbnail_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        let pics_count = unprocessed_pics.len();
        if let Err(e) = sender.output(GeneratePreviewsOutput::Started(pics_count)){
            println!("Failed sending gen started: {:?}", e);
        }

        unprocessed_pics
            .par_iter()
            .for_each(|pic| {
                let result = block_on(async {previewer.get_extra(&pic.picture_id, &pic.path).await});
                let result = result.and_then(|extra| repo.clone().update(&pic.picture_id, &extra));

                if let Err(e) = result {
                    println!("Failed add_preview: {:?}", e);
                } else if let Err(e) = sender.output(GeneratePreviewsOutput::Generated) {
                    println!("Failed sending GeneratePreviewsOutput::Generated: {:?}", e);
                }
            });

        println!("Generated {} previews in {} seconds.", pics_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(GeneratePreviewsOutput::Completed) {
            println!("Failed sending GeneratePreviewsOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for GeneratePreviews {
    type Init = (fotema_core::photo::Previewer, fotema_core::photo::Repository);
    type Input = GeneratePreviewsInput;
    type Output = GeneratePreviewsOutput;

    fn init((previewer, repo): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        let model = GeneratePreviews {
            previewer,
            repo,
        };
        model
    }


    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            GeneratePreviewsInput::Start => {
                println!("Generating previews...");
                let repo = self.repo.clone();
                let previewer = self.previewer.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = GeneratePreviews::update_previews(repo, previewer, &sender) {
                        println!("Failed to update previews: {}", e);
                    }
                });
            }
        };
    }
}
