// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use std::path::{Path, PathBuf};

use candle_nn::{Module, VarBuilder};
use candle_transformers::models::mobileone;
use candle_core::Device;

use crate::face::{
    blaze_face,
    blaze_face::utilities,
};

#[derive(Debug, Clone)]
pub struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
pub struct Face {
    thumbnail: PathBuf,
    bounds: Rect,
}

pub struct FaceExtractor {
    base_face_path: PathBuf,
    base_model_path: PathBuf,
    model_type: blaze_face::ModelType,
    model: blaze_face::BlazeFace,
    device: Device,
}

impl FaceExtractor {
    pub fn build(base_face_path: &Path, base_model_path: &Path) -> Result<FaceExtractor> {
        let base_face_path = PathBuf::from(base_face_path).join("photo_faces");
        std::fs::create_dir_all(&base_face_path)?;

        // FIXME would be nice to try GPU and fallback to CPU.
        let device = Device::Cpu;


        // TODO try out front model when we know the front camera was used.
        let model_type = blaze_face::ModelType::Back;

        let model = utilities::load_model(base_model_path, model_type, 0.6, 0.3, &device)?;

        Ok(FaceExtractor {
            base_face_path,
            base_model_path: base_model_path.into(),
            model_type,
            model,
            device,
        })
    }

    /// Identify faces in a photo and return a vector of paths of extracted face images.
    pub fn extract_faces(&self, picture_id: &PictureId, picture_path: &Path) -> Result<Vec<Face>> {

        let image = utilities::load_image(picture_path, self.model_type)?;
        let image_tensor = utilities::convert_image_to_tensor(&image, &self.device)?;
        dbg!(&image_tensor);

        let detections = self.model.predict_on_image(&image_tensor)?;

        let detections = blaze_face::FaceDetection::from_tensors(
            detections.first()
                .unwrap()
                .clone(),
        )?;

        println!("{:?} faces detections: {:?}", detections.len(), detections);

        todo!("do this")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_faces() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let image_path = Path::new(dir).join("resources/test/Sandow.jpg");
        let base_face_path = Path::new(".");
        let base_model_path = Path::new("/var/home/david/Projects/fotema/core/src/face/blaze_face/data");

        let extractor = FaceExtractor::build(&base_face_path, &base_model_path).unwrap();
        extractor.extract_faces(&PictureId::new(0), &image_path).unwrap();
    }
}
