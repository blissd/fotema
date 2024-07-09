// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use std::path::{Path, PathBuf};

use rust_faces::{
    viz, BlazeFaceParams, FaceDetection, FaceDetectorBuilder, InferParams, Provider, ToArray3,
    ToRgb8,
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
    confidence: f32,
}

pub struct FaceExtractor {
    base_face_path: PathBuf,
}

impl FaceExtractor {
    pub fn build(base_face_path: &Path, base_model_path: &Path) -> Result<FaceExtractor> {
        let base_face_path = PathBuf::from(base_face_path).join("photo_faces");
        std::fs::create_dir_all(&base_face_path)?;

        Ok(FaceExtractor { base_face_path })
    }

    /// Identify faces in a photo and return a vector of paths of extracted face images.
    pub fn extract_faces(&self, picture_id: &PictureId, picture_path: &Path) -> Result<Vec<Face>> {
        let face_detector =
            FaceDetectorBuilder::new(FaceDetection::BlazeFace640(BlazeFaceParams::default()))
                .download()
                .infer_params(InferParams {
                    provider: Provider::OrtCpu,
                    intra_threads: Some(5),
                    ..Default::default()
                })
                .build()?;

        let original_image = image::open(picture_path)?;

        let image = original_image.clone().into_rgb8().into_array3();

        let faces = face_detector.detect(image.view().into_dyn())?;

        println!("{:?} faces detections: {:?}", faces.len(), faces);

        let faces = faces
            .into_iter()
            .map(|f| {
                let bounds = Rect {
                    x: f.rect.x as u32,
                    y: f.rect.y as u32,
                    width: f.rect.width as u32,
                    height: f.rect.height as u32,
                };

                let thumbnail =
                    original_image.crop_imm(bounds.x, bounds.y, bounds.width, bounds.height);
                thumbnail.save("out.png");

                Face {
                    thumbnail: PathBuf::new(),
                    bounds,
                    confidence: f.confidence,
                }
            })
            .collect();

        Ok(faces)
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
        let base_model_path =
            Path::new("/var/home/david/Projects/fotema/core/src/face/blaze_face/data");

        let extractor = FaceExtractor::build(&base_face_path, &base_model_path).unwrap();
        extractor
            .extract_faces(&PictureId::new(0), &image_path)
            .unwrap();
    }
}
