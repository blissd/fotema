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
use tracing::{error, info};

use fotema_core::machine_learning::face_recognizer::FaceRecognizer;
use fotema_core::people;
use fotema_core::people::model::{DetectedFace, PersonForRecognition};

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};

#[derive(Debug)]
pub enum PhotoRecognizeFacesTaskInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoRecognizeFacesTaskOutput {
    // Face recognition has started.
    Started,

    // Face recognition has completed
    Completed,
}

#[derive(Clone)]
pub struct PhotoRecognizeFacesTask {
    // Stop flag
    stop: Arc<AtomicBool>,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: people::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,

    cache_dir: PathBuf,
}

impl PhotoRecognizeFacesTask {
    fn recognize(&self, sender: ComponentSender<Self>) -> Result<()> {
        let start = std::time::Instant::now();

        // Faces that still need an SFace embedding (for clustering in the
        // "unknown people" view). Computed for every unnamed face, independent
        // of whether any people have been named yet.
        let model_path = FaceRecognizer::ensure_model(&self.cache_dir)?;
        let done = self.repo.faces_with_embedding()?;
        let need_embedding: Vec<DetectedFace> = self
            .repo
            .find_unknown_faces()?
            .into_iter()
            .filter(|f| !done.contains(&f.face_id.id()))
            .collect();

        let people: Vec<PersonForRecognition> = self
            .repo
            .find_people_for_recognition()?
            .into_iter()
            .collect();

        info!(
            "Face recognition: {} faces need embeddings, {} named people",
            need_embedding.len(),
            people.len()
        );

        if need_embedding.is_empty() && people.is_empty() {
            let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);
            return Ok(());
        }

        let _ = sender.output(PhotoRecognizeFacesTaskOutput::Started);

        // ---- Embedding pass (GPU via OpenCL when available, else CPU). One
        // recognizer instance per rayon thread: the model is loaded once per
        // thread rather than once per face, and OpenCV's FaceRecognizerSF is not
        // thread-safe so it must not be shared.
        if !need_embedding.is_empty() {
            self.progress_monitor.emit(ProgressMonitorInput::Start(
                TaskName::RecognizeFaces,
                need_embedding.len(),
            ));

            need_embedding
                .into_par_iter()
                .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
                .for_each_init(
                    || FaceRecognizer::new_sface(&model_path).ok(),
                    |recognizer, face| {
                        if let Some(rec) = recognizer.as_mut() {
                            match FaceRecognizer::embedding(rec, &face) {
                                Ok(embedding) => {
                                    let _ = self
                                        .repo
                                        .store_face_embedding(face.face_id, &embedding);
                                }
                                Err(e) => error!(
                                    "Failed computing embedding for face {}: {:?}",
                                    face.face_id, e
                                ),
                            }
                        }
                        self.progress_monitor.emit(ProgressMonitorInput::Advance);
                    },
                );

            self.progress_monitor.emit(ProgressMonitorInput::Complete);
        }

        // ---- Recognition pass: match unnamed faces against named people.
        if !people.is_empty() {
            let min_recognized_at = people.iter().map(|x| x.recognized_at).min().unwrap();

            let unprocessed: Vec<DetectedFace> = self
                .repo
                .find_unknown_faces()?
                .into_iter()
                .filter(|unknown_face| unknown_face.detected_at > min_recognized_at)
                .collect();

            if !unprocessed.is_empty() {
                self.progress_monitor.emit(ProgressMonitorInput::Start(
                    TaskName::RecognizeFaces,
                    unprocessed.len(),
                ));

                let recognizer = FaceRecognizer::build(&self.cache_dir, people.clone())?;

                unprocessed
                    .into_par_iter()
                    .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
                    .for_each(|unknown_face| {
                        let is_match = recognizer.recognize(&unknown_face);
                        if let Ok(Some(person_id)) = is_match {
                            info!(
                                "Face {} looks like person {}",
                                unknown_face.face_id, person_id
                            );
                            let mut repo = self.repo.clone();
                            let result =
                                repo.mark_as_person_unconfirmed(unknown_face.face_id, person_id);
                            if let Err(e) = result {
                                error!(
                                    "Failed marking face {} as person: {:?}",
                                    unknown_face.face_id, e
                                );
                            }
                        }

                        self.progress_monitor.emit(ProgressMonitorInput::Advance);
                    });

                self.progress_monitor.emit(ProgressMonitorInput::Complete);
            }

            let mut repo = self.repo.clone();
            for person in people {
                if let Err(e) = repo.mark_face_recognition_complete(person.person_id) {
                    error!(
                        "Failed marking face recognition complete for person {}: {:?}",
                        person.person_id, e
                    );
                }
            }
        }

        info!(
            "Recognized/embedded faces in {} seconds.",
            start.elapsed().as_secs()
        );

        let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);

        Ok(())
    }
}

impl Worker for PhotoRecognizeFacesTask {
    type Init = (
        Arc<AtomicBool>,
        PathBuf,
        people::Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = PhotoRecognizeFacesTaskInput;
    type Output = PhotoRecognizeFacesTaskOutput;

    fn init(
        (stop, cache_dir, repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        PhotoRecognizeFacesTask {
            stop,
            cache_dir,
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoRecognizeFacesTaskInput::Start => {
                info!("Recognizing photo faces...");
                let this = self.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = this.recognize(sender.clone()) {
                        error!("Failed to recognize photo faces: {}", e);
                        let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);
                    }
                });
            }
        };
    }
}
