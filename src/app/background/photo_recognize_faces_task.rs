// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{error, info};

use fotema_core::machine_learning::face_recognizer::{FaceEmbedder, FaceRecognizer};
use fotema_core::people;
use fotema_core::people::model::{DetectedFace, PersonForRecognition};
use fotema_core::photo;
use fotema_core::{FaceId, FlatpakPathBuf, PersonId, PictureId};

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};

#[derive(Debug)]
pub enum PhotoRecognizeFacesTaskInput {
    Start,

    /// Re-read XMP person tags for the whole library (clears the per-picture
    /// import marker first), then run the normal recognition pass.
    RescanFaceTags,

    /// Lightweight, low-priority pass triggered after the user names a face:
    /// match unnamed faces to named people purely from the embeddings already
    /// stored in the DB (no model, no image reads, no inference). Lets a newly
    /// named person propagate across the library in the background.
    RecognizeNow,
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
        negatives: &HashMap<i64, HashSet<i64>>,
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
            // XMP names are a recommendation: assign them unconfirmed so the
            // user can override them.
            let result = match people_repo.find_person_id_by_name(name) {
                Ok(Some(person_id)) => {
                    // Respect a prior "not this person" rejection.
                    if negatives
                        .get(&face.face_id.id())
                        .is_some_and(|r| r.contains(&person_id.id()))
                    {
                        continue;
                    }
                    people_repo.mark_as_person_unconfirmed(face.face_id, person_id)
                }
                Ok(None) => people_repo.add_person_unconfirmed(face.face_id, name),
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
        let negatives = self.repo.find_negative_associations().unwrap_or_default();
        let mut processed: Vec<PictureId> = Vec::with_capacity(groups.len());
        let mut imported = 0usize;

        for (picture_id, _path, faces) in groups {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }
            processed.push(picture_id);
            let regions = self.photo_repo.find_face_tags(picture_id).unwrap_or_default();
            imported += Self::assign_regions(faces, regions, &mut people_repo, &negatives);
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
        let negatives = self.repo.find_negative_associations().unwrap_or_default();
        let mut refreshed: Vec<(PictureId, Vec<photo::face_tags::FaceTag>)> = Vec::new();
        let mut processed: Vec<PictureId> = Vec::with_capacity(groups.len());
        let mut imported = 0usize;

        for (picture_id, path, faces) in groups {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }
            processed.push(picture_id);
            let regions = photo::face_tags::read_face_tags(&path.sandbox_path);
            imported += Self::assign_regions(faces, regions.clone(), &mut people_repo, &negatives);
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

        // Faces that still need an ArcFace embedding (for clustering in the
        // "unknown people" view). Computed for every unnamed face, independent
        // of whether any people have been named yet.
        let (sface_path, arcface_path) = FaceRecognizer::ensure_models(&self.cache_dir)?;
        // All faces (named or not) lacking a current ArcFace embedding. Covers
        // the one-time recompute after switching from SFace.
        let need_embedding: Vec<DetectedFace> = self.repo.find_faces_needing_embedding()?;

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

        // Cap concurrency: each worker thread loads its own ~166MB ArcFace net,
        // so a bounded pool keeps memory (and GPU memory) in check. Embeddings
        // are a one-off recompute, so throughput is not critical.
        let pool = rayon::ThreadPoolBuilder::new().num_threads(4).build()?;

        // ---- Embedding pass (GPU via OpenCL when available, else CPU). One
        // embedder instance per worker thread: the models are loaded once per
        // thread rather than once per face, and are not thread-safe.
        if !need_embedding.is_empty() {
            self.progress_monitor.emit(ProgressMonitorInput::Start(
                TaskName::RecognizeFaces,
                need_embedding.len(),
            ));

            pool.install(|| {
            need_embedding
                .into_par_iter()
                .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
                .for_each_init(
                    || FaceEmbedder::new(&sface_path, &arcface_path).ok(),
                    |embedder, face| {
                        if let Some(emb) = embedder.as_mut() {
                            match emb.embedding(&face) {
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
            });

            self.progress_monitor.emit(ProgressMonitorInput::Complete);
        }

        // ---- Recognition pass: match unnamed faces to named people from the
        // embeddings already stored in the DB — pure cosine comparison, no
        // model load, image read or inference.
        if !people.is_empty() {
            match self.recognize_from_embeddings() {
                Ok(n) if n > 0 => info!("Recognised {} face(s) from stored embeddings.", n),
                Ok(_) => {}
                Err(e) => error!("Embedding recognition failed: {:?}", e),
            }
        }

        info!(
            "Recognized/embedded faces in {} seconds.",
            start.elapsed().as_secs()
        );

        let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);

        Ok(())
    }

    /// Match unnamed faces to named people purely from the embeddings already
    /// stored in the DB. No model, no image reads, no inference — just a cosine
    /// comparison against each named person's reference faces. Cheap enough to
    /// run after every naming. Returns the number of faces newly named.
    fn recognize_from_embeddings(&self) -> Result<usize> {
        // Reference faces: confirmed, named, with a current embedding.
        let references = self.repo.find_recognition_references()?;
        if references.is_empty() {
            return Ok(0);
        }

        // Faces detected at or before *every* person's last recognition were
        // already considered; skip them. A freshly named person has
        // recognized_at = epoch, so it still sees the whole library the first time.
        let min_recognized_at = references.iter().map(|r| r.recognized_at).min().unwrap();

        let unnamed: Vec<people::UnnamedEmbedding> = self
            .repo
            .find_unnamed_with_embeddings()?
            .into_iter()
            .filter(|face| face.detected_at > min_recognized_at)
            .collect();

        // "This face is NOT person X" rejections, respected so corrected
        // mistakes don't come back (negative learning).
        let negatives = self.repo.find_negative_associations().unwrap_or_default();

        // Precision-leaning cosine threshold (mirrors the auto-recognition default).
        const THRESHOLD: f32 = 0.42;

        let assignments: Vec<(FaceId, PersonId)> = unnamed
            .par_iter()
            .take_any_while(|_| !self.stop.load(Ordering::Relaxed))
            .filter_map(|face| {
                let rejected = negatives.get(&face.face_id.id());
                let mut best_person: Option<PersonId> = None;
                let mut best_cos = THRESHOLD;
                for reference in &references {
                    // Per-person timing: a face is only matched to people whose
                    // last recognition is at or before the face's detection.
                    if reference.recognized_at > face.detected_at {
                        continue;
                    }
                    if rejected.is_some_and(|set| set.contains(&reference.person_id.id())) {
                        continue;
                    }
                    let cos =
                        people::Repository::cosine_normalized(&face.embedding, &reference.embedding);
                    if cos > best_cos {
                        best_cos = cos;
                        best_person = Some(reference.person_id);
                    }
                }
                best_person.map(|person_id| (face.face_id, person_id))
            })
            .collect();

        let count = assignments.len();
        let mut repo = self.repo.clone();
        for (face_id, person_id) in assignments {
            info!("Face {} looks like person {}", face_id, person_id);
            // Auto-recognised matches are unconfirmed — overridable by the user.
            if let Err(e) = repo.mark_as_person_unconfirmed(face_id, person_id) {
                error!("Failed marking face {} as person: {:?}", face_id, e);
            }
        }

        // Advance recognized_at for every named person so the next pass only
        // considers faces detected since this run.
        let mut person_ids: Vec<i64> = references.iter().map(|r| r.person_id.id()).collect();
        person_ids.sort_unstable();
        person_ids.dedup();
        for id in person_ids {
            if let Err(e) = repo.mark_face_recognition_complete(PersonId::new(id)) {
                error!("Failed marking recognition complete for person {}: {:?}", id, e);
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
            PhotoRecognizeFacesTaskInput::RecognizeNow => {
                info!("Propagating named people across the library (background)...");
                let this = self.clone();

                rayon::spawn(move || {
                    let _ = sender.output(PhotoRecognizeFacesTaskOutput::Started);
                    match this.recognize_from_embeddings() {
                        Ok(n) if n > 0 => info!("Background recognition named {} face(s).", n),
                        Ok(_) => {}
                        Err(e) => error!("Background recognition failed: {:?}", e),
                    }
                    let _ = sender.output(PhotoRecognizeFacesTaskOutput::Completed);
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
