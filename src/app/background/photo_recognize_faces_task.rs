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

use fotema_core::machine_learning::face_recognizer::FaceEmbedder;
use fotema_core::people;

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};

/// Cosine similarity above which an unknown face is taken to be a named person.
/// ArcFace separates identities well; this leans towards precision.
const COSINE_THRESHOLD: f32 = 0.42;

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

        // Named reference faces and the current embeddings in the DB. The
        // recognition pass compares these vectors; it does not run inference.
        let references = self.repo.find_recognition_references()?;

        // All faces (named or not) lacking a current ArcFace embedding. Covers
        // the one-time recompute after switching from SFace.
        let need_embedding = self.repo.find_faces_needing_embedding()?;

        info!(
            "Face recognition: {} faces need embeddings, {} reference faces",
            need_embedding.len(),
            references.len()
        );

        if need_embedding.is_empty() && references.is_empty() {
            let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);
            return Ok(());
        }

        let _ = sender.output(PhotoRecognizeFacesTaskOutput::Started);

        // Cap concurrency: each worker thread loads its own ~166 MB ArcFace net,
        // so a bounded pool keeps memory (and GPU memory) in check.
        if !need_embedding.is_empty() {
            let (sface_path, arcface_path) = FaceEmbedder::ensure_models(&self.cache_dir)?;

            self.progress_monitor.emit(ProgressMonitorInput::Start(
                TaskName::RecognizeFaces,
                need_embedding.len(),
            ));

            let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build()?;
            pool.install(|| {
                need_embedding
                    .into_par_iter()
                    .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
                    .for_each_init(
                        || FaceEmbedder::new(&sface_path, &arcface_path).ok(),
                        |embedder, face| {
                            if let Some(embedder) = embedder.as_mut() {
                                match embedder.embedding(&face) {
                                    Ok(embedding) => {
                                        if let Err(e) =
                                            self.repo.store_face_embedding(face.face_id, &embedding)
                                        {
                                            error!(
                                                "Failed storing embedding for face {}: {:?}",
                                                face.face_id, e
                                            );
                                        }
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
            });

            self.progress_monitor.emit(ProgressMonitorInput::Complete);
        }

        if !references.is_empty() {
            self.recognize_from_embeddings(&references)?;
        }

        info!(
            "Recognized/embedded faces in {} seconds.",
            start.elapsed().as_secs()
        );

        let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);

        Ok(())
    }

    /// Match unnamed faces to named people using only the embeddings already
    /// stored in the database. Pure cosine comparison: no model load, image
    /// read or inference per face.
    fn recognize_from_embeddings(
        &self,
        references: &[people::RecognitionReference],
    ) -> Result<usize> {
        let min_recognized_at = references
            .iter()
            .map(|reference| reference.recognized_at)
            .min()
            .expect("references is not empty");

        let unnamed = self
            .repo
            .find_unnamed_with_embeddings()?
            .into_iter()
            .filter(|face| face.detected_at > min_recognized_at)
            .collect::<Vec<_>>();

        let assignments: Vec<(fotema_core::FaceId, fotema_core::PersonId)> = unnamed
            .par_iter()
            .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
            .filter_map(|face| {
                let mut best: Option<(fotema_core::PersonId, f32)> = None;
                for reference in references {
                    // Only match against people known before this face was detected.
                    if reference.recognized_at > face.detected_at {
                        continue;
                    }
                    let score = people::Repository::cosine_normalized(
                        &face.embedding,
                        &reference.embedding,
                    );
                    if score >= COSINE_THRESHOLD
                        && best.is_none_or(|(_, best_score)| score > best_score)
                    {
                        best = Some((reference.person_id, score));
                    }
                }
                best.map(|(person_id, score)| {
                    info!(
                        "Face {} looks like person {} (score {:.3})",
                        face.face_id, person_id, score
                    );
                    (face.face_id, person_id)
                })
            })
            .collect();

        let count = assignments.len();
        let mut repo = self.repo.clone();
        for (face_id, person_id) in assignments {
            if let Err(e) = repo.mark_as_person_unconfirmed(face_id, person_id) {
                error!("Failed marking face {} as person: {:?}", face_id, e);
            }
        }

        let mut person_ids: Vec<i64> = references
            .iter()
            .map(|reference| reference.person_id.id())
            .collect();
        person_ids.sort_unstable();
        person_ids.dedup();
        for id in person_ids {
            if let Err(e) = repo.mark_face_recognition_complete(fotema_core::PersonId::new(id)) {
                error!(
                    "Failed marking recognition complete for person {}: {:?}",
                    id, e
                );
            }
        }

        Ok(count)
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
