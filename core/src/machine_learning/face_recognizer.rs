use anyhow::Result;

use opencv::core::Mat;
use opencv::imgcodecs;
use opencv::objdetect::{FaceRecognizerSF, FaceRecognizerSF_DisType};
use opencv::prelude::*;

use crate::people::model::DetectedFace;

pub struct FaceRecognizer {
    /// Feature derived from a known person's face.
    //model_path: PathBuf,
    person_face_feature: Mat,
}

impl FaceRecognizer {
    //const COSINE_SIMILAR_THRESH: f64 = 0.363;
    const L2NORM_SIMILAR_THRESH: f64 = 1.128;
    const MODEL_PATH: &'static str =
        &"/var/home/david/Pictures/face_recognition_sface_2021dec.onnx";

    pub fn build(person: &DetectedFace) -> Result<Self> {
        //face_recognition_sface_2021dec.onnx
        // FIXME download and cache at runtime
        //let model_path = Path::new("/var/home/david/Pictures/face_recognition_sface_2021dec.onnx");
        let mut face_recognizer = FaceRecognizerSF::create_def(Self::MODEL_PATH, "")?;

        let face_img = imgcodecs::imread_def(&person.face_path.to_string_lossy())?;

        let face_landarks = person.landmarks_as_mat();

        let mut aligned_face = Mat::default();
        face_recognizer.align_crop(&face_img, &face_landarks, &mut aligned_face)?;

        // Run feature extraction with given aligned_face
        let mut face_features = Mat::default();
        face_recognizer.feature(&aligned_face, &mut face_features)?;

        Ok(Self {
            // model_path: PathBuf::from("/var/home/david/Pictures/face_recognition_sface_2021dec.onnx"),
            person_face_feature: face_features,
        })
    }

    pub fn recognize(&mut self, unknown_face: &DetectedFace) -> Result<bool> {
        let mut face_recognizer = FaceRecognizerSF::create_def(Self::MODEL_PATH, "")?;

        let face_img = imgcodecs::imread_def(&unknown_face.face_path.to_string_lossy())?;

        let face_landmarks = unknown_face.landmarks_as_mat();

        let mut aligned_face = Mat::default();
        face_recognizer.align_crop(&face_img, &face_landmarks, &mut aligned_face)?;

        // Run feature extraction with given aligned_face
        let mut face_features = Mat::default();
        face_recognizer.feature(&aligned_face, &mut face_features)?;

        /*
        let cos_score = face_recognizer.match_(
            &self.person_face_feature,
            &face_features,
            FaceRecognizerSF_DisType::FR_COSINE.into(),
        )?;
        */

        let l2_score = face_recognizer.match_(
            &self.person_face_feature,
            &face_features,
            FaceRecognizerSF_DisType::FR_NORM_L2.into(),
        )?;

        // The internet said the l2norm should give better results than the cosine.
        Ok(l2_score <= Self::L2NORM_SIMILAR_THRESH)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recognize() {
        //let dir = env!("CARGO_MANIFEST_DIR");
        let base_dir = "/var/home/david/.var/app/app.fotema.Fotema.Devel/cache/app.fotema.Fotema.Devel/photo_faces";
        let person_face = Path::new(base_dir).join("0001/1038/2_original.png");

        let fr = FaceRecognizer::build(&person_face).unwrap();
    }
}
