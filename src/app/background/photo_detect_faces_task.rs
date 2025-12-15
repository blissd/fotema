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
use fotema_core::people::FaceDetectionCandidate;
use fotema_core::photo;
use fotema_core::photo::PictureId;
use fotema_core::thumbnailify::Thumbnailer;

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};
use deadpool::managed;

#[derive(Debug)]
enum PoolError { Fail }

struct FaceDetectorPoolManager{
    /// Base directory for storing photo faces
    faces_base_dir: PathBuf,
    thumbnailer: Thumbnailer,
}

impl managed::Manager for FaceDetectorPoolManager {
    type Type = FaceExtractor;
    type Error = Error;

    async fn create(&self) -> Result<FaceExtractor, Error> {
        FaceExtractor::build(&self.faces_base_dir, self.thumbnailer.clone())
    }

    async fn recycle(&self, _: &mut FaceExtractor, _: &managed::Metrics) -> managed::RecycleResult<Error> {
        Ok(())
    }
}

type FaceDetectorPool = managed::Pool<FaceDetectorPoolManager>;


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
    thumbnailer: Thumbnailer,

    photo_repo: photo::Repository,
    people_repo: people::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoDetectFacesTask {
    fn detect_for_one(&self, sender: ComponentSender<Self>, picture_id: PictureId) -> Result<()> {
        self.people_repo.delete_faces(picture_id)?;
        let result = self.photo_repo.get_face_detection_candidate(&picture_id)?;
        if let Some(candidate) = result {
            let unprocessed = vec![candidate];
            self.detect(sender, unprocessed)
        } else {
            Err(anyhow!("No file to scan"))
        }
    }

    fn detect_for_all(&self, sender: ComponentSender<Self>) -> Result<()> {
        let unprocessed: Vec<FaceDetectionCandidate> = self
            .photo_repo
            .find_face_detection_candidates()?
            .into_iter()
            .filter(|candidate| candidate.sandbox_path.exists())
            .collect();

        self.detect(sender, unprocessed)
    }

    fn detect(
        &self,
        sender: ComponentSender<Self>,
        unprocessed: Vec<FaceDetectionCandidate>,
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
        //let mut extractor = FaceExtractor::build(&self.faces_base_dir, self.thumbnailer.clone())?;

        // Create a face detector to trigger download of face detection models.
        // We must do this before using the object pool and parallel processing, otherwise
        // multiple threads will try to download the same model.
        // FIXME add a method to the face detection library to download models.
        let _ = FaceExtractor::build(&self.faces_base_dir, self.thumbnailer.clone());

        let detector_pool_manager = FaceDetectorPoolManager {
            faces_base_dir: self.faces_base_dir.clone(),
            thumbnailer: self.thumbnailer.clone(),
        };
        let detector_pool = FaceDetectorPool::builder(detector_pool_manager).build()?;

        unprocessed
            .par_iter()
            .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
            .for_each(|candidate| {
                let mut repo = self.people_repo.clone();

                // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
                // an error but doesn't panic.
                let result = block_on(async {
                    // FIXME unwrap
                    let mut detector = detector_pool.get().await.unwrap();
                    detector.extract_faces(&candidate).await
                    })
                    .and_then(|faces| repo.clone().add_face_scans(&candidate.picture_id, &faces));

                if result.is_err() {
                    error!(
                        "Failed detecting faces: Photo path: {:?}. Error: {:?}",
                        candidate.sandbox_path, result
                    );
                    let _ = repo.mark_face_scan_broken(&candidate.picture_id);
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
        Thumbnailer,
        photo::Repository,
        people::Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = PhotoDetectFacesTaskInput;
    type Output = PhotoDetectFacesTaskOutput;

    fn init(
        (stop, faces_base_dir, thumbnailer, photo_repo, people_repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        PhotoDetectFacesTask {
            stop,
            faces_base_dir,
            thumbnailer,
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
                    if let Err(e) = this.detect_for_one(sender.clone(), picture_id) {
                        error!("Failed to extract photo faces: {}", e);
                        let _ = sender.output(PhotoDetectFacesTaskOutput::Completed);
                    }
                });
            }
        };
    }
}
