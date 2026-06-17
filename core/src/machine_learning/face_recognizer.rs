// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

use opencv::core::Mat;
use opencv::imgcodecs;
use opencv::objdetect::{FaceRecognizerSF, FaceRecognizerSF_DisType};
use opencv::prelude::*;

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};

use tracing::info;

use crate::people::model::{DetectedFace, PersonForRecognition, PersonId};

pub struct FaceRecognizer {
    /// Person recognition data and a opencv matrix of aligned face features.
    people: Vec<(PersonForRecognition, Mat)>,

    /// Path to OpenCV face recognition model
    model_path: PathBuf,
}

impl FaceRecognizer {
    //const COSINE_SIMILAR_THRESH: f64 = 0.363;
    const L2NORM_SIMILAR_THRESH: f64 = 1.128;

    const MODEL_URL: &'static str = "https://github.com/blissd/fotema-opencv_zoo/raw/fotema-1.0/models/face_recognition_sface/face_recognition_sface_2021dec.onnx";

    pub fn build(cache_dir: &Path, people: Vec<PersonForRecognition>) -> Result<Self> {
        let model_path = {
            let base_path = cache_dir.join("opencv_models");
            std::fs::create_dir_all(&base_path)?;
            base_path.join("face_recognition_sface_2021dec.onnx")
        };

        Self::download_model(Self::MODEL_URL, &model_path)?;

        let mut recognizer = Self {
            people: vec![],
            model_path,
        };

        for person in people {
            // WARNING cannot re-use recognizer. MUST use a separate one for each person.
            let mut opencv_face_recognizer =
                FaceRecognizerSF::create_def(&recognizer.model_path.to_string_lossy(), "")?;

            let face_img = imgcodecs::imread_def(&person.face.face_path.to_string_lossy())?;

            let face_landarks = person.face.landmarks_as_mat();

            let mut aligned_face = Mat::default();
            opencv_face_recognizer.align_crop(&face_img, &face_landarks, &mut aligned_face)?;

            // Run feature extraction with given aligned_face
            let mut face_features = Mat::default();
            opencv_face_recognizer.feature(&aligned_face, &mut face_features)?;

            recognizer.people.push((person, face_features));
        }

        Ok(recognizer)
    }

    pub fn recognize(&self, unknown_face: &DetectedFace) -> Result<Option<PersonId>> {
        let mut face_recognizer =
            FaceRecognizerSF::create_def(&self.model_path.to_string_lossy(), "")?;

        let face_img = imgcodecs::imread_def(&unknown_face.face_path.to_string_lossy())?;

        let face_landmarks = unknown_face.landmarks_as_mat();

        let mut aligned_face = Mat::default();
        face_recognizer.align_crop(&face_img, &face_landmarks, &mut aligned_face)?;

        let mut face_features = Mat::default();
        face_recognizer.feature(&aligned_face, &mut face_features)?;

        let best_person_and_score = self
            .people
            .iter()
            .filter(|(p, _)| p.recognized_at <= unknown_face.detected_at)
            .map(|(person, person_face_features)| {
                let l2_score = face_recognizer.match_(
                    &person_face_features,
                    &face_features,
                    FaceRecognizerSF_DisType::FR_NORM_L2.into(),
                );
                (
                    person,
                    l2_score.unwrap_or(Self::L2NORM_SIMILAR_THRESH + 100.0),
                )
            })
            // FIXME do we need to filter out NaNs?
            .min_by_key(|x| (x.1 * 10000.0) as i32); // f64 doesn't implement Ord.

        if let Some((person, l2_score)) = best_person_and_score {
            // The internet said the l2norm should give better results than the cosine.
            if l2_score <= Self::L2NORM_SIMILAR_THRESH {
                return Ok(Some(person.person_id));
            }
        }

        Ok(None)
    }

    /// Ensure the SFace recognition model is downloaded and return its path.
    pub fn ensure_model(cache_dir: &Path) -> Result<PathBuf> {
        let model_path = {
            let base_path = cache_dir.join("opencv_models");
            std::fs::create_dir_all(&base_path)?;
            base_path.join("face_recognition_sface_2021dec.onnx")
        };
        Self::download_model(Self::MODEL_URL, &model_path)?;
        Ok(model_path)
    }

    /// Create an SFace recognizer, preferring the GPU via OpenCL and falling
    /// back to CPU. One instance may be reused for many `embedding()` calls on a
    /// single thread, but must NOT be shared across threads (OpenCV's
    /// FaceRecognizerSF is not thread-safe).
    pub fn new_sface(model_path: &Path) -> Result<opencv::core::Ptr<FaceRecognizerSF>> {
        let model = model_path.to_string_lossy();
        if opencv::core::have_opencl().unwrap_or(false) {
            if let Ok(r) = FaceRecognizerSF::create(
                &model,
                "",
                opencv::dnn::DNN_BACKEND_OPENCV,
                opencv::dnn::DNN_TARGET_OPENCL,
            ) {
                info!("Face recognizer using OpenCL (GPU) acceleration.");
                return Ok(r);
            }
            info!("OpenCL face recognizer unavailable; falling back to CPU.");
        }
        Ok(FaceRecognizerSF::create(
            &model,
            "",
            opencv::dnn::DNN_BACKEND_OPENCV,
            opencv::dnn::DNN_TARGET_CPU,
        )?)
    }

    /// Compute the SFace embedding (feature vector) for a detected face. Reuse
    /// `recognizer` across faces on the same thread to avoid reloading the model.
    pub fn embedding(
        recognizer: &mut opencv::core::Ptr<FaceRecognizerSF>,
        face: &DetectedFace,
    ) -> Result<Vec<f32>> {
        let face_img = imgcodecs::imread_def(&face.face_path.to_string_lossy())?;
        let landmarks = face.landmarks_as_mat();
        let mut aligned = Mat::default();
        recognizer.align_crop(&face_img, &landmarks, &mut aligned)?;
        let mut features = Mat::default();
        recognizer.feature(&aligned, &mut features)?;
        let data: &[f32] = features.data_typed()?;
        Ok(data.to_vec())
    }

    fn download_model(url: &str, destination: &Path) -> Result<()> {
        if destination.exists() {
            info!("Face recognition model already downloaded.");
            return Ok(());
        }

        info!("Downloading face recognition model from {}", url);
        info!("Model is approximately 40MB.");

        let headers = {
            let mut headers = HeaderMap::new();
            headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
            headers
        };

        let client = reqwest::blocking::Client::new();
        let mut response = client.get(url).headers(headers).send()?;

        if response.status().is_success() {
            let tmp_path = destination.with_extension("tmp");
            let tmp_file = File::create(&tmp_path)?;
            let mut writer = BufWriter::new(tmp_file);
            while let Ok(bytes_read) = response.copy_to(&mut writer) {
                if bytes_read == 0 {
                    break;
                }
            }
            info!("Face recognition model successfully downloaded.");
            std::fs::rename(tmp_path, destination)?;

            Ok(())
        } else {
            Err(anyhow!(
                "Failed to download face recognition model: {}",
                response.status()
            ))
        }
    }
}

// (The previous `test_recognize` unit test was removed: it used the pre-2.x
// `FaceRecognizer::build` signature and a hard-coded developer path, so it no
// longer compiled or ran.)
