// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Result, anyhow};

use opencv::core::Mat;
use opencv::imgcodecs;
use opencv::objdetect::{FaceRecognizerSF, FaceRecognizerSF_DisType};
use opencv::prelude::*;

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};
use reqwest::redirect::Policy;

use tracing::info;

use crate::people::model::{DetectedFace, PersonForRecognition, PersonId};

/// Time limit for the connection setup.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Time limit for the whole transfer. The blocking client has no limit for a
/// single read, so this limit covers the complete download. A stalled server
/// then no longer blocks the calling thread forever.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// Upper limit for HTTP redirects.
const MAX_REDIRECTS: usize = 10;

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

            let face_img = imgcodecs::imread_def(&person.face.face_path)?;

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

        let face_img = imgcodecs::imread_def(&unknown_face.face_path)?;

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

        let client = reqwest::blocking::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TRANSFER_TIMEOUT)
            .redirect(Policy::limited(MAX_REDIRECTS))
            .build()?;

        let mut response = client.get(url).headers(headers).send()?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to download face recognition model: {}",
                response.status()
            ));
        }

        // The expected length, if the server reports it. A short transfer then
        // gives an error instead of a truncated model file.
        let expected_len = response.content_length();

        // The temporary file carries the process id. Thus two processes never
        // write into the same file.
        let tmp_path = destination.with_extension(format!("{}.tmp", std::process::id()));

        let written = match Self::write_body(&mut response, &tmp_path) {
            Ok(written) => written,
            Err(e) => {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(e);
            }
        };

        if let Some(expected) = expected_len {
            if written != expected {
                let _ = std::fs::remove_file(&tmp_path);
                return Err(anyhow!(
                    "Face recognition model is incomplete: got {} bytes, expected {} bytes",
                    written,
                    expected
                ));
            }
        }

        std::fs::rename(&tmp_path, destination)?;
        info!("Face recognition model successfully downloaded.");

        Ok(())
    }

    /// Write the response body to `tmp_path` and return the number of bytes.
    ///
    /// The body is read once, in blocks. The previous code called `copy_to` in a
    /// loop. That is wrong: the first call already consumes the whole body, and
    /// a later call only returns 0. A read error also ended that loop without an
    /// error, so a truncated file became the final model file.
    fn write_body(response: &mut reqwest::blocking::Response, tmp_path: &Path) -> Result<u64> {
        let tmp_file = File::create(tmp_path)?;
        let mut writer = BufWriter::new(tmp_file);

        let mut buffer = vec![0u8; 256 * 1024];
        let mut written: u64 = 0;
        loop {
            let n = response.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n])?;
            written += n as u64;
        }

        // Flush the buffer and the file. Without these steps the rename can
        // publish a file that is still incomplete on disk.
        writer.flush()?;
        let file = writer
            .into_inner()
            .map_err(|e| anyhow!("Failed to flush face recognition model: {e}"))?;
        file.sync_all()?;

        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::people::model::{FaceId, Rect};
    use std::path::PathBuf;

    #[test]
    fn test_recognize() {
        let person_face = DetectedFace {
            face_id: FaceId::new(1),
            face_path: PathBuf::from(
                "/var/home/david/.var/app/app.fotema.Fotema.Devel/cache/app.fotema.Fotema.Devel/photo_faces/0003/3027/0_blaze_face_640_original.png",
            ),
            bounds: Rect {
                x: 0.,
                y: 0.,
                width: 100.,
                height: 100.,
            },

            right_eye: (20., 10.),
            left_eye: (10., 10.),
            nose: (15., 15.),
            right_mouth_corner: (20., 20.),
            left_mouth_corner: (10., 20.),

            confidence: 0.98,
        };

        let _ = FaceRecognizer::build(&person_face).unwrap();
    }
}
