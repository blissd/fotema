// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;

use std::path::PathBuf;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::executor::block_on;
use tracing::{error, info};

use fotema_core::machine_learning::face_extractor::FaceExtractor;
use fotema_core::people;
use fotema_core::photo;
use fotema_core::photo::PictureId;

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};

#[derive(Debug)]
pub enum PhotoDetectFacesTaskInput {
    DetectForAllPictures,
    DetectForOnePicture(PictureId),
}

#[derive(Debug)]
pub enum PhotoDetectFacesTaskOutput {
    // Face detection has started.
    Started,

    // Face detection has completed
    Completed,
}

#[derive(Clone)]
pub struct PhotoDetectFacesTask {
    // Stop flag
    stop: Arc<AtomicBool>,

    /// Base directory for storing photo faces
    faces_base_dir: PathBuf,

    photo_repo: photo::Repository,
    people_repo: people::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoDetectFacesTask {
    fn detect_for_one(&self, sender: ComponentSender<Self>, picture_id: PictureId) -> Result<()> {
        self.people_repo.delete_faces(picture_id)?;
        let result = self.photo_repo.get_picture_path(picture_id)?;
        if let Some(picture_path) = result {
            let unprocessed = vec![(picture_id, picture_path)];
            self.detect(sender, unprocessed)
        } else {
            Err(anyhow!("No file to scan"))
        }
    }

    fn detect_for_all(&self, sender: ComponentSender<Self>) -> Result<()> {
        let unprocessed: Vec<(PictureId, PathBuf)> = self
            .photo_repo
            .find_need_face_scan()?
            .into_iter()
            .filter(|(_, path)| path.exists())
            .collect();

        self.detect(sender, unprocessed)
    }

    fn detect(
        &self,
        sender: ComponentSender<Self>,
        unprocessed: Vec<(PictureId, PathBuf)>,
    ) -> Result<()> {
        let start = std::time::Instant::now();

        let count = unprocessed.len();
        info!("Found {} photos as candidates for face detection", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoDetectFacesTaskOutput::Completed);
            return Ok(());
        }

        let _ = sender.output(PhotoDetectFacesTaskOutput::Started);

        self.progress_monitor
            .emit(ProgressMonitorInput::Start(TaskName::DetectFaces, count));

        // Must build face extractor here rather than in Boostrap's init function because
        // the face detection models will be downloaded on creation and that mustn't happen
        // on the main thread.
        // Also, this has the advantage of extractor being dropped after use, which means
        // the face detection models will be unloaded from memory.
        let extractor = FaceExtractor::build(&self.faces_base_dir)?;

        unprocessed
            //.into_iter()
            .par_iter()
            .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
            .for_each(|(picture_id, path)| {
                let mut repo = self.people_repo.clone();

                // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
                // an error but doesn't panic.
                let result = block_on(async { extractor.extract_faces(picture_id, path).await })
                    .and_then(|faces| repo.clone().add_face_scans(picture_id, &faces));

                if result.is_err() {
                    error!(
                        "Failed detecting faces: Photo path: {:?}. Error: {:?}",
                        path, result
                    );
                    let _ = repo.mark_face_scan_broken(picture_id);
                }

                self.progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!(
            "Detected faces in {} photos in {} seconds.",
            count,
            start.elapsed().as_secs()
        );

        self.progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PhotoDetectFacesTaskOutput::Completed);

        Ok(())
    }
}

impl Worker for PhotoDetectFacesTask {
    type Init = (
        Arc<AtomicBool>,
        PathBuf,
        photo::Repository,
        people::Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = PhotoDetectFacesTaskInput;
    type Output = PhotoDetectFacesTaskOutput;

    fn init(
        (stop, faces_base_dir, photo_repo, people_repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        PhotoDetectFacesTask {
            stop,
            faces_base_dir,
            photo_repo,
            people_repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoDetectFacesTaskInput::DetectForAllPictures => {
                info!("Extracting faces for all pictures...");
                let this = self.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = this.detect_for_all(sender) {
                        error!("Failed to extract photo faces: {}", e);
                    }
                });
            }

            PhotoDetectFacesTaskInput::DetectForOnePicture(picture_id) => {
                info!("Extracting faces for one picture...");
                let this = self.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = this.detect_for_one(sender, picture_id) {
                        error!("Failed to extract photo faces: {}", e);
                    }
                });
            }
        };
    }
}
