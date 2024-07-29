use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;

use opencv::core::{Mat, Ptr};
use opencv::imgcodecs;
use opencv::objdetect::{FaceRecognizerSF, FaceRecognizerSF_DisType};
use opencv::prelude::*;

use crate::people::model::DetectedFace;

pub struct FaceRecognizer {
    //model_path: PathBuf,
    face_recognizer: Ptr<FaceRecognizerSF>,

    /// Feature derived from a known person's face.
    person_face_feature: Mat,
}

impl FaceRecognizer {
    const cosine_similar_thresh: f64 = 0.363;
    const l2norm_similar_thresh: f64 = 1.128;

    pub fn build(person: &DetectedFace) -> Result<Self> {
        //face_recognition_sface_2021dec.onnx
        // FIXME download and cache at runtime
        /*
                let fd_model_path = "/var/home/david/Pictures/face_detection_yunet_2023mar.onnx";

                let mut detector = FaceDetectorYN::create(
                    &fd_model_path,
                    "",
                    Size::new(320, 320),
                    0.9,
                    0.3,
                    5000,
                    0,
                    0,
                )?;

                let image_width = (f64::from(image1.cols()) * 1.0) as i32;
                let image_height = (f64::from(image1.rows()) * 1.0) as i32;
                println!("width = {}, height = {}", image_width, image_height);

                // Set input size before inference
                detector.set_input_size(image1.size()?)?;

                let mut faces1 = Mat::default();
                detector.detect(&image1, &mut faces1)?;

                println!("{} faces found", faces1.rows());
                for i in 0..15 {
                    println!("{}={:?}, ", i, faces1.at_2d::<f32>(0, i));
                }
        */
        let model_path = Path::new("/var/home/david/Pictures/face_recognition_sface_2021dec.onnx");
        let mut face_recognizer = FaceRecognizerSF::create_def(&model_path.to_string_lossy(), "")?;

        let face_img = imgcodecs::imread_def(&person.face_path.to_string_lossy())?;

        let face_landarks = person.landmarks_as_mat();

        let mut aligned_face = Mat::default();
        face_recognizer.align_crop(&face_img, &face_landarks, &mut aligned_face)?;

        // Run feature extraction with given aligned_face
        let mut face_features = Mat::default();
        face_recognizer.feature(&aligned_face, &mut face_features)?;

        Ok(Self {
            //     model_path: PathBuf::from(model_path),
            face_recognizer,
            person_face_feature: face_features,
        })
    }

    pub fn recognize(&mut self, unknown_face: &DetectedFace) -> Result<bool> {
        let face_img = imgcodecs::imread_def(&unknown_face.face_path.to_string_lossy())?;

        let face_landmarks = unknown_face.landmarks_as_mat();

        let mut aligned_face = Mat::default();
        self.face_recognizer
            .align_crop(&face_img, &face_landmarks, &mut aligned_face)?;

        // Run feature extraction with given aligned_face
        let mut face_features = Mat::default();
        self.face_recognizer
            .feature(&aligned_face, &mut face_features)?;

        let cos_score = self.face_recognizer.match_(
            &self.person_face_feature,
            &face_features,
            FaceRecognizerSF_DisType::FR_COSINE.into(),
        )?;
        if cos_score >= Self::cosine_similar_thresh {
            println!("They have the same identity;");
        } else {
            println!("They have different identities;");
        }
        println!(
				"Cosine Similarity: {cos_score}, threshold: {}. (higher value means higher similarity, max 1.0)",
				Self::cosine_similar_thresh,
			);

        let l2_score = self.face_recognizer.match_(
            &self.person_face_feature,
            &face_features,
            FaceRecognizerSF_DisType::FR_NORM_L2.into(),
        )?;
        if l2_score <= Self::l2norm_similar_thresh {
            println!("They have the same identity;");
        } else {
            println!("They have different identities.");
        }
        println!(
				"NormL2 Distance: {l2_score}, threshold: {}. (lower value means higher similarity, min 0.0)",
				Self::l2norm_similar_thresh,
			);

        Ok(false)
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
