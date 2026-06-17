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
use fotema_core::photo;
use fotema_core::{FlatpakPathBuf, PictureId};

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};

#[derive(Debug)]
pub enum PhotoRecognizeFacesTaskInput {
    Start,

    /// Re-read XMP person tags for the whole library (clears the per-picture
    /// import marker first), then run the normal recognition pass.
    RescanFaceTags,
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

    // Used to read picture paths for importing names from photo XMP metadata.
    photo_repo: photo::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,

    cache_dir: PathBuf,
}

impl PhotoRecognizeFacesTask {
    /// Import person names embedded in photos' XMP (MWG regions / Microsoft
    /// people tags) and assign them to detected faces. Runs at most once per
    /// picture (tracked by `pictures.face_tags_imported`). Conservative: only
    /// assigns when the number of named regions equals the number of unnamed
    /// faces, matched left-to-right. Returns the number of faces named.
    fn import_face_tags(&self) -> Result<usize> {
        let rows = self.photo_repo.find_unnamed_faces_with_pictures()?;
        if rows.is_empty() {
            return Ok(0);
        }

        // Rows are ordered by picture_id; group consecutive rows per picture.
        let mut groups: Vec<(PictureId, FlatpakPathBuf, Vec<DetectedFace>)> = Vec::new();
        for (picture_id, path, face) in rows {
            match groups.last_mut() {
                Some(last) if last.0 == picture_id => last.2.push(face),
                _ => groups.push((picture_id, path, vec![face])),
            }
        }

        let mut people_repo = self.repo.clone();
        let mut processed: Vec<PictureId> = Vec::with_capacity(groups.len());
        let mut imported = 0usize;

        for (picture_id, path, mut faces) in groups {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }
            processed.push(picture_id);

            let mut tags = photo::face_tags::read_face_tags(&path.sandbox_path);

            // Only act when we can match confidently: one named region per
            // unnamed face, paired left-to-right by horizontal centre.
            if tags.is_empty() || tags.len() != faces.len() {
                continue;
            }

            let by_x = |a: f32, b: f32| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
            tags.sort_by(|a, b| by_x(a.center_x, b.center_x));
            faces.sort_by(|a, b| {
                by_x(
                    a.bounds.x + a.bounds.width / 2.0,
                    b.bounds.x + b.bounds.width / 2.0,
                )
            });

            for (tag, face) in tags.iter().zip(faces.iter()) {
                let name = tag.name.trim();
                if name.is_empty() {
                    continue;
                }
                let result = match people_repo.find_person_id_by_name(name) {
                    Ok(Some(person_id)) => people_repo.mark_as_person(face.face_id, person_id),
                    Ok(None) => people_repo.add_person(face.face_id, name),
                    Err(e) => {
                        error!("Looking up person '{}' failed: {:?}", name, e);
                        continue;
                    }
                };
                match result {
                    Ok(()) => imported += 1,
                    Err(e) => error!(
                        "Importing face {} as '{}' failed: {:?}",
                        face.face_id, name, e
                    ),
                }
            }
        }

        if !processed.is_empty() {
            let mut photo_repo = self.photo_repo.clone();
            if let Err(e) = photo_repo.mark_face_tags_imported(&processed) {
                error!("Failed marking pictures as tag-imported: {:?}", e);
            }
        }

        Ok(imported)
    }

    fn recognize(&self, sender: ComponentSender<Self>) -> Result<()> {
        let start = std::time::Instant::now();

        // Import names already present in the photos' metadata before computing
        // embeddings, so imported faces drop out of the unnamed set.
        match self.import_face_tags() {
            Ok(n) if n > 0 => info!("Imported {} face name(s) from photo metadata.", n),
            Ok(_) => {}
            Err(e) => error!("Face-tag import failed: {:?}", e),
        }

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
        photo::Repository,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = PhotoRecognizeFacesTaskInput;
    type Output = PhotoRecognizeFacesTaskOutput;

    fn init(
        (stop, cache_dir, repo, photo_repo, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        PhotoRecognizeFacesTask {
            stop,
            cache_dir,
            repo,
            photo_repo,
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
            PhotoRecognizeFacesTaskInput::RescanFaceTags => {
                info!("Re-scanning all photos for embedded person tags...");
                let this = self.clone();

                rayon::spawn(move || {
                    let mut photo_repo = this.photo_repo.clone();
                    if let Err(e) = photo_repo.reset_face_tags_imported() {
                        error!("Failed resetting face-tag import markers: {:?}", e);
                    }
                    if let Err(e) = this.recognize(sender.clone()) {
                        error!("Failed to recognize photo faces: {}", e);
                        let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);
                    }
                });
            }
        };
    }
}
