// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use std::sync::{Arc, Mutex};
use photos_core::Result;
use rayon::prelude::*;
use futures::future;
use futures::Future;
use futures::stream::FuturesUnordered;
use futures::stream::StreamExt;
use itertools::Itertools;


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
    previewer: photos_core::Previewer,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: Arc<Mutex<photos_core::Repository>>,
}

impl GeneratePreviews {

    async fn update_previews(&self, sender: &AsyncComponentSender<Self>) -> Result<()> {
        let start = std::time::Instant::now();

        let unprocessed_pics: Vec<photos_core::repo::Picture> = self.repo
            .lock()
            .unwrap()
            .all()?
            .into_iter()
            .filter(|pic| !pic.square_preview_path.as_ref().is_some_and(|p| p.exists()))
            .collect();

        let pics_count = unprocessed_pics.len();

        if let Err(e) = sender.output(GeneratePreviewsOutput::Started(pics_count)){
            println!("Failed sending gen started: {:?}", e);
        }

        let futs: Vec<_> = unprocessed_pics.into_iter().map(|pic| {
                self.previewer.set_preview(pic.clone())
            }).collect();

        let s: String = FuturesUnordered::from_iter(futs).fuse().for_each_concurrent(2, |item| async move {
            if let Ok(pic) = item {
                let result = self.repo.lock().unwrap().add_preview(&pic);
                if let Err(e) = result {
                    println!("Failed add_preview: {:?}", e);
                } else if let Err(e) = sender.output(GeneratePreviewsOutput::Generated) {
                    println!("Failed sending GeneratePreviewsOutput::Generated: {:?}", e);
                }
            }
        });


        /*for chunk in unprocessed_pics.chunks(10) {

            let futs = chunk.into_iter().map(|pic| {
                self.previewer.set_preview(pic.clone())
            });

*/
/*
            println!("Before blocking on stream");
            loop {
                let item = futs.next().await;
                let Some(item) = item else {
                    break;
                };

                if let Ok(pic) = item {
                    let result = self.repo.lock().unwrap().add_preview(&pic);
                    if let Err(e) = result {
                        println!("Failed add_preview: {:?}", e);
                    } else if let Err(e) = sender.output(GeneratePreviewsOutput::Generated) {
                        println!("Failed sending GeneratePreviewsOutput::Generated: {:?}", e);
                    }
                } else {
                    break;
                }
            }
        }
*/

        //let pics = future::join_all(pics).await;
/*
        for pic in pics {
            if let Ok(pic) = pic {
                let result = self.repo.lock().unwrap().add_preview(&pic);
                if let Err(e) = result {
                    println!("Failed add_preview: {:?}", e);
                } else if let Err(e) = sender.output(GeneratePreviewsOutput::Generated) {
                    println!("Failed sending GeneratePreviewsOutput::Generated: {:?}", e);
                }
            }
        }
        */

        println!("Generated {} previews in {} seconds.", pics_count, start.elapsed().as_secs());

        if let Err(e) = sender.output(GeneratePreviewsOutput::Completed) {
            println!("Failed sending GeneratePreviewsOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

// I would like to implement an async relm4::Worker instead of SimpleAsyncComponent.
//#[relm4::component(pub async)]
impl SimpleAsyncComponent for GeneratePreviews {
    type Init = (photos_core::Previewer, Arc<Mutex<photos_core::Repository>>);
    type Input = GeneratePreviewsInput;
    type Output = GeneratePreviewsOutput;
    type Root = ();
    type Widgets = ();

    fn init_root() -> Self::Root {
        ()
    }

    async fn init(
        (previewer, repo): Self::Init,
        _root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {
        let model = GeneratePreviews {
            previewer,
            repo,
        };
        AsyncComponentParts { model, widgets: () }
    }


    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            GeneratePreviewsInput::Start => {
                println!("Generating previews...");
                if let Err(e) = self.update_previews(&sender).await {
                    println!("Failed to update previews: {}", e);
                }
            }
        };
    }
}
