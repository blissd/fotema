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
    /// Unnamed, non-ignored faces grouped by their picture (id + path), for
    /// pictures whose tags have not yet been matched.
    fn unnamed_faces_by_picture(
        &self,
    ) -> Result<Vec<(PictureId, FlatpakPathBuf, Vec<DetectedFace>)>> {
        // Rows are ordered by picture_id; group consecutive rows per picture.
        let mut groups: Vec<(PictureId, FlatpakPathBuf, Vec<DetectedFace>)> = Vec::new();
        for (picture_id, path, face) in self.photo_repo.find_unnamed_faces_with_pictures()? {
            match groups.last_mut() {
                Some(last) if last.0 == picture_id => last.2.push(face),
                _ => groups.push((picture_id, path, vec![face])),
            }
        }
        Ok(groups)
    }

    /// Assign named regions to a picture's unnamed faces. Conservative: only acts
    /// when the number of named regions equals the number of unnamed faces,
    /// paired left-to-right by horizontal centre. Creates or reuses a person by
    /// name. Returns the number of faces named.
    fn assign_regions(
        mut faces: Vec<DetectedFace>,
        mut regions: Vec<photo::face_tags::FaceTag>,
        people_repo: &mut people::Repository,
    ) -> usize {
        if regions.is_empty() || regions.len() != faces.len() {
            return 0;
        }

        let by_x = |a: f32, b: f32| a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
        regions.sort_by(|a, b| by_x(a.center_x, b.center_x));
        faces.sort_by(|a, b| {
            by_x(
                a.bounds.x + a.bounds.width / 2.0,
                b.bounds.x + b.bounds.width / 2.0,
            )
        });

        let mut named = 0usize;
        for (region, face) in regions.iter().zip(faces.iter()) {
            let name = region.name.trim();
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
                Ok(()) => named += 1,
                Err(e) => error!(
                    "Importing face {} as '{}' failed: {:?}",
                    face.face_id, name, e
                ),
            }
        }
        named
    }

    /// Match XMP person tags (already cached in the DB during enrich) to detected
    /// faces. No file reads. Runs at most once per picture
    /// (`pictures.face_tags_imported`). Returns the number of faces named.
    fn import_face_tags(&self) -> Result<usize> {
        let groups = self.unnamed_faces_by_picture()?;
        if groups.is_empty() {
            return Ok(0);
        }

        let mut people_repo = self.repo.clone();
        let mut processed: Vec<PictureId> = Vec::with_capacity(groups.len());
        let mut imported = 0usize;

        for (picture_id, _path, faces) in groups {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }
            processed.push(picture_id);
            let regions = self.photo_repo.find_face_tags(picture_id).unwrap_or_default();
            imported += Self::assign_regions(faces, regions, &mut people_repo);
        }

        if !processed.is_empty() {
            if let Err(e) = self.photo_repo.clone().mark_face_tags_imported(&processed) {
                error!("Failed marking pictures as tag-imported: {:?}", e);
            }
        }

        Ok(imported)
    }

    /// Retrospective re-scan: re-read every relevant photo's XMP from disk (to
    /// pick up tags added by other apps since the last enrich), refresh the
    /// cached regions, and re-match. Used by the manual menu action.
    fn rescan_face_tags(&self) -> Result<usize> {
        let _ = self.photo_repo.clone().reset_face_tags_imported();

        let groups = self.unnamed_faces_by_picture()?;
        if groups.is_empty() {
            return Ok(0);
        }

        let mut people_repo = self.repo.clone();
        let mut refreshed: Vec<(PictureId, Vec<photo::face_tags::FaceTag>)> = Vec::new();
        let mut processed: Vec<PictureId> = Vec::with_capacity(groups.len());
        let mut imported = 0usize;

        for (picture_id, path, faces) in groups {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }
            processed.push(picture_id);
            let regions = photo::face_tags::read_face_tags(&path.sandbox_path);
            imported += Self::assign_regions(faces, regions.clone(), &mut people_repo);
            refreshed.push((picture_id, regions));
        }

        let mut photo_repo = self.photo_repo.clone();
        let _ = photo_repo.add_face_tags(&refreshed);
        let _ = photo_repo.mark_face_tags_imported(&processed);

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
                    let _ = sender.output(PhotoRecognizeFacesTaskOutput::Started);
                    match this.rescan_face_tags() {
                        Ok(n) => info!("Re-scan named {} face(s) from photo metadata.", n),
                        Err(e) => error!("Face-tag re-scan failed: {:?}", e),
                    }
                    let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);
                });
            }
        };
    }
}
