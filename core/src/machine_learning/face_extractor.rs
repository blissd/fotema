// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::result::Result::Ok;

use rust_faces::{
    BlazeFaceParams, FaceDetection, FaceDetectorBuilder, InferParams, MtCnnParams, Provider,
    ToArray3,
};

use gdk4::prelude::TextureExt;
use image::DynamicImage;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct Face {
    /// Path to thumbnail generated from face bounds.
    /// Normalized to be square and expanded to capture the whole head.
    pub thumbnail_path: PathBuf,

    /// Image cropped from bounds returned by face detection algorithm
    pub bounds_path: PathBuf,

    /// Bounds of detected face.
    pub bounds: Rect,

    /// Confidence (0.0 to 1.0) that the detected face is actually a face.
    pub confidence: f32,

    /// Facial landmarks.
    /// I _think_ this is right eye, left eye, nose, right mouth corner, left mouth corner.
    /// Note that left/right are from the subject's perspective, not the observer.
    landmarks: Option<Vec<(f32, f32)>>,
}

impl Face {
    fn landmark(&self, index: usize) -> Option<(u32, u32)> {
        self.landmarks
            .as_ref()
            .filter(|x| x.len() == 5)
            .map(|x| (x[index].0 as u32, x[index].1 as u32))
    }

    pub fn right_eye(&self) -> Option<(u32, u32)> {
        self.landmark(0)
    }

    pub fn left_eye(&self) -> Option<(u32, u32)> {
        self.landmark(1)
    }

    pub fn nose(&self) -> Option<(u32, u32)> {
        self.landmark(2)
    }

    pub fn right_mouth_corner(&self) -> Option<(u32, u32)> {
        self.landmark(3)
    }

    pub fn left_mouth_corner(&self) -> Option<(u32, u32)> {
        self.landmark(4)
    }
}

pub struct FaceExtractor {
    base_path: PathBuf,
    back_model: Box<dyn rust_faces::FaceDetector>,
    front_model: Box<dyn rust_faces::FaceDetector>,
}

impl FaceExtractor {
    pub fn build(base_path: &Path) -> Result<FaceExtractor> {
        let base_path = PathBuf::from(base_path).join("photo_faces");
        std::fs::create_dir_all(&base_path)?;

        let back_model =
            FaceDetectorBuilder::new(FaceDetection::BlazeFace640(BlazeFaceParams::default()))
                .download()
                .infer_params(InferParams {
                    provider: Provider::OrtCpu,
                    intra_threads: Some(5),
                    ..Default::default()
                })
                .build()?;

        let front_model =
            FaceDetectorBuilder::new(FaceDetection::BlazeFace320(BlazeFaceParams::default()))
                .download()
                .infer_params(InferParams {
                    provider: Provider::OrtCpu,
                    intra_threads: Some(5),
                    ..Default::default()
                })
                .build()?;

        Ok(FaceExtractor {
            base_path,
            back_model,
            front_model,
        })
    }

    /// Identify faces in a photo and return a vector of paths of extracted face images.
    pub async fn extract_faces(
        &self,
        picture_id: &PictureId,
        picture_path: &Path,
    ) -> Result<Vec<Face>> {
        let original_image = FaceExtractor::open_image(picture_path).await?;

        let image = original_image.clone().into_rgb8().into_array3();

        let faces = panic::catch_unwind(AssertUnwindSafe(|| {
            self.back_model.detect(image.view().into_dyn())
        }));

        let faces = match faces {
            Ok(Ok(ref fs)) if !fs.is_empty() => faces,
            //Ok(Ok(_)) => faces,
            _ => {
                println!("Failed extracting faces with back model, using front model");
                panic::catch_unwind(AssertUnwindSafe(|| {
                    self.front_model.detect(image.view().into_dyn())
                }))
            }
        };

        let Ok(Ok(faces)) = faces else {
            // If we got an err, then there was a panic.
            // If we got Ok(Err(e)) there wasn't a panic, but we still failed.
            let err = match faces {
                Ok(Err(e)) => Err(anyhow!("Failed detecting faces: {:?}", e)),
                Err(e) => Err(anyhow!("Panicked detecting faces: {:?}", e)),
                _ => Err(anyhow!("Other error detecting faces")),
            };

            return err;
        };

        debug!(
            "Picture {} has {} faces. Found: {:?}",
            picture_id,
            faces.len(),
            faces
        );

        let base_path = {
            // Create a directory per 1000 thumbnails
            let partition = (picture_id.id() / 1000) as i32;
            let partition = format!("{:0>4}", partition);
            let file_name = format!("{}", picture_id);
            self.base_path.join(partition).join(file_name)
        };

        let faces = faces
            .into_iter()
            .enumerate()
            .map(|(index, f)| {
                if !base_path.exists() {
                    let _ = std::fs::create_dir_all(&base_path);
                }

                // Extract face and save to thumbnail.
                // The bounding box is pretty tight, so make it a bit bigger.
                // Also, make the box a square.

                let longest: u32 =
                    (std::cmp::max(f.rect.width as u32, f.rect.height as u32) as f32 * 1.6) as u32;
                let half_longest: u32 = longest / 2;

                let (centre_x, centre_y) = if let Some(ref landmarks) = f.landmarks {
                    // If we have landmarks, then the first two are the right and left eyes.
                    // Use the midpoint between the eyes as the centre of the thumbnail.
                    let x = ((landmarks[0].0 + landmarks[1].0) / 2.0) as u32;
                    let y = ((landmarks[0].1 + landmarks[1].1) / 2.0) as u32;
                    (x, y)
                } else {
                    let x = (f.rect.x + (f.rect.width / 2.0)) as u32;
                    let y = (f.rect.y + (f.rect.height / 2.0)) as u32;
                    (x, y)
                };

                // Don't panic when x/y would be < zero
                let x: u32 = centre_x.checked_sub(half_longest).unwrap_or(0);
                let y: u32 = centre_y.checked_sub(half_longest).unwrap_or(0);

                let thumbnail = original_image.crop_imm(x, y, longest, longest);
                let thumbnail_path = base_path.join(format!("{}_thumbnail.png", index));
                let _ = thumbnail.save(&thumbnail_path);

                let bounds = Rect {
                    x: f.rect.x as u32,
                    y: f.rect.y as u32,
                    width: f.rect.width as u32,
                    height: f.rect.height as u32,
                };

                let bounds_img =
                    original_image.crop_imm(bounds.x, bounds.y, bounds.width, bounds.height);

                let bounds_path = base_path.join(format!("{}_original.png", index));
                let _ = bounds_img.save(&bounds_path);

                Face {
                    thumbnail_path,
                    bounds_path,
                    bounds,
                    confidence: f.confidence,
                    landmarks: f.landmarks,
                }
            })
            .collect();

        Ok(faces)
    }

    async fn open_image(source_path: &Path) -> Result<DynamicImage> {
        let file = gio::File::for_path(source_path);

        let image = glycin::Loader::new(file).load().await?;

        let frame = image.next_frame().await?;

        let png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        // FIXME can we avoid this step of saving to the file system and just
        // load the image from memory?
        frame.texture.save_to_png(png_file.path())?;

        Ok(image::open(png_file.path())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_faces() {
        let dir = env!("CARGO_MANIFEST_DIR");
        // let image_path = Path::new(dir).join("resources/test/Sandow.jpg");
        let image_path = Path::new(
            "/var/home/david/Pictures/Ente/Recents/0400B8FC-B0FB-413A-BDDA-428499E5905C.JPG",
        );
        let base_face_path = Path::new(".");

        let extractor = FaceExtractor::build(&base_face_path).unwrap();
        extractor
            .extract_faces(&PictureId::new(0), &image_path)
            .unwrap();
    }
}
