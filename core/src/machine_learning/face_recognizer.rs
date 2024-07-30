// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;

use opencv::core::Mat;
use opencv::imgcodecs;
use opencv::objdetect::{FaceRecognizerSF, FaceRecognizerSF_DisType};
use opencv::prelude::*;

use crate::people::model::{DetectedFace, PersonForRecognition, PersonId};

pub struct FaceRecognizer {
    /// Person recognition data and a opencv matrix of aligned face features.
    people: Vec<(PersonForRecognition, Mat)>,
}

impl FaceRecognizer {
    //const COSINE_SIMILAR_THRESH: f64 = 0.363;
    const L2NORM_SIMILAR_THRESH: f64 = 1.128;

    //face_recognition_sface_2021dec.onnx
    // FIXME download and cache at runtime
    const MODEL_PATH: &'static str =
        &"/var/home/david/Pictures/face_recognition_sface_2021dec.onnx";

    pub fn build(people: Vec<PersonForRecognition>) -> Result<Self> {
        let mut recognizer = Self { people: vec![] };

        for person in people {
            // WARNING cannot re-use recognizer. MUST use a separate one for each person.
            let mut opencv_face_recognizer = FaceRecognizerSF::create_def(Self::MODEL_PATH, "")?;

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
        let mut face_recognizer = FaceRecognizerSF::create_def(Self::MODEL_PATH, "")?;

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
            .min_by_key(|x| (x.1 * 10000.0) as i32); // f64 doesn't implement Ord.

        if let Some((person, l2_score)) = best_person_and_score {
            // The internet said the l2norm should give better results than the cosine.
            if l2_score <= Self::L2NORM_SIMILAR_THRESH {
                return Ok(Some(person.person_id));
            }
        }

        Ok(None)
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
            face_path: PathBuf::from("/var/home/david/.var/app/app.fotema.Fotema.Devel/cache/app.fotema.Fotema.Devel/photo_faces/0003/3027/0_blaze_face_640_original.png"),
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
