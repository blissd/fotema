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
use fotema_core::machine_learning::face_extractor::ExtractMode;
use fotema_core::people;
use fotema_core::photo::PictureId;

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
};


#[derive(Debug)]
pub enum PhotoDetectFacesInput {
    DetectForAllPictures,
    DetectForOnePicture(PictureId),
}

#[derive(Debug)]
pub enum PhotoDetectFacesOutput {
    // Face detection has started.
    Started,

    // Face detection has completed
    Completed(usize),

}

#[derive(Clone)]
pub struct PhotoDetectFaces {
    extractor: Arc<FaceExtractor>,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: people::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoDetectFaces {

    fn detect_for_one(&self, sender: ComponentSender<Self>, picture_id: PictureId) -> Result<()> {
        self.repo.delete_faces(picture_id)?;
        let result = self.repo.get_file_to_scan(picture_id)?;
        if let Some(picture_path) = result {
            let unprocessed = vec![(picture_id, picture_path)];
            self.detect(sender, ExtractMode::Heavyweight, unprocessed)
        } else {
            Err(anyhow!("No file to scan"))
        }
    }

    fn detect_for_all(&self, sender: ComponentSender<Self>) -> Result<()> {
        let unprocessed: Vec<(PictureId, PathBuf)> = self.repo
            .find_need_face_scan()?
            .into_iter()
            .filter(|(_, path)| path.exists())
            .collect();

        self.detect(sender, ExtractMode::Lightweight, unprocessed)
    }

    fn detect(&self, sender: ComponentSender<Self>, extract_mode: ExtractMode, unprocessed: Vec<(PictureId, PathBuf)>) -> Result<()> {
        let start = std::time::Instant::now();

        let count = unprocessed.len();
         info!("Found {} photos as candidates for face detection", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoDetectFacesOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PhotoDetectFacesOutput::Started);

        self.progress_monitor.emit(ProgressMonitorInput::Start(TaskName::DetectFaces, count));

        unprocessed
            //.into_iter()
            .par_iter()
            .for_each(|(picture_id, path)| {
                let mut repo = self.repo.clone();

                // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
                // an error but doesn't panic.
                let result = block_on(async {
                        self.extractor.extract_faces(&picture_id, &path, extract_mode).await
                    }).and_then(|faces| repo.clone().add_face_scans(&picture_id, &faces));

                if result.is_err() {
                    error!("Failed detecting faces: Photo path: {:?}. Error: {:?}", path, result);
                    let _ = repo.mark_face_scan_broken(&picture_id);
                }

                self.progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!("Detected faces in {} photos in {} seconds.", count, start.elapsed().as_secs());

        self.progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PhotoDetectFacesOutput::Completed(count));

        Ok(())
    }
}

impl Worker for PhotoDetectFaces {
    type Init = (FaceExtractor, people::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = PhotoDetectFacesInput;
    type Output = PhotoDetectFacesOutput;

    fn init((extractor, repo, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        PhotoDetectFaces {
            extractor: Arc::new(extractor),
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoDetectFacesInput::DetectForAllPictures => {
                info!("Extracting faces for all pictures...");
                let this = self.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = this.detect_for_all(sender) {
                        error!("Failed to extract photo faces: {}", e);
                    }
                });
            },

            PhotoDetectFacesInput::DetectForOnePicture(picture_id) => {
                info!("Extracting faces for one picture...");
                let this = self.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = this.detect_for_one(sender, picture_id) {
                        error!("Failed to extract photo faces: {}", e);
                    }
                });
            },
        };
    }
}
