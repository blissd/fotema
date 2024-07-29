// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use relm4::Reducer;
use rayon::prelude::*;
use anyhow::*;
use std::sync::Arc;
use std::result::Result::Ok;
use std::path::PathBuf;
use tracing::{error, info};
use futures::executor::block_on;

use fotema_core::machine_learning::face_extractor::FaceExtractor;
use fotema_core::people;
use fotema_core::photo::PictureId;

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
};


#[derive(Debug)]
pub enum PhotoRecognizeFacesInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoRecognizeFacesOutput {
    // Face recognition has started.
    Started,

    // Face recognition has completed
    Completed(usize),

}

#[derive(Clone)]
pub struct PhotoRecognizeFaces {
    extractor: Arc<FaceExtractor>,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: people::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoRecognizeFaces {

    fn detect(&self, sender: ComponentSender<Self>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed: Vec<(PictureId, PathBuf)> = self.repo
            .find_need_face_scan()?
            .into_iter()
            .filter(|(_, path)| path.exists())
            .collect();

        let count = unprocessed.len();
         info!("Found {} photos as candidates for face recognition", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoRecognizeFacesOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PhotoRecognizeFacesOutput::Started);

        self.progress_monitor.emit(ProgressMonitorInput::Start(TaskName::DetectFaces, count));

        unprocessed
            //.into_iter()
            .par_iter()
            .for_each(|(picture_id, path)| {
                let mut repo = self.repo.clone();

                // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
                // an error but doesn't panic.
                let result = block_on(async {
                        self.extractor.extract_faces(&picture_id, &path).await
                    }).and_then(|faces| repo.clone().add_face_scans(&picture_id, &faces));

                if result.is_err() {
                    error!("Failed detecting faces: Photo path: {:?}. Error: {:?}", path, result);
                    let _ = repo.mark_face_scan_broken(&picture_id);
                }

                self.progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!("Detected faces in {} photos in {} seconds.", count, start.elapsed().as_secs());

        self.progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PhotoRecognizeFacesOutput::Completed(count));

        Ok(())
    }
}

impl Worker for PhotoRecognizeFaces {
    type Init = (FaceExtractor, people::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = PhotoRecognizeFacesInput;
    type Output = PhotoRecognizeFacesOutput;

    fn init((extractor, repo, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        PhotoRecognizeFaces {
            extractor: Arc::new(extractor),
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoRecognizeFacesInput::Start => {
                info!("Recognizing photo faces...");
                let this = self.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = this.detect(sender) {
                        error!("Failed to recognize photo faces: {}", e);
                    }
                });
            }
        };
    }
}
