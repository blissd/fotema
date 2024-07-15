// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use std::path::{Path, PathBuf};
use std::result::Result::Ok;

use rust_faces::{
    BlazeFaceParams, Face as DetectedFace, FaceDetection, FaceDetectorBuilder, InferParams,
    MtCnnParams, Provider, ToArray3,
};

use gdk4::prelude::TextureExt;
use image::DynamicImage;
use tracing::{debug, error};

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

    /// Name of model that detected this face.
    pub model_name: String,
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

    /// I think this is the "back model" trained on
    /// photos taken by the back camera of phones.
    blaze_face_640_model: Box<dyn rust_faces::FaceDetector>,

    /// I think this is the "front model" trained on
    /// photos taken by the selfie camera of phones.
    blaze_face_320_model: Box<dyn rust_faces::FaceDetector>,

    /// An alternative model with good results, but much slower than
    /// BlazeFace.
    mtcnn_model: Box<dyn rust_faces::FaceDetector>,
    is_mtcnn_enabled: bool,
}

impl FaceExtractor {
    pub fn build(base_path: &Path) -> Result<FaceExtractor> {
        let base_path = PathBuf::from(base_path).join("photo_faces");
        std::fs::create_dir_all(&base_path)?;

        let bz_params = BlazeFaceParams {
            score_threshold: 0.95, // confidence match is a face
            ..BlazeFaceParams::default()
        };

        let blaze_face_640_model =
            FaceDetectorBuilder::new(FaceDetection::BlazeFace640(bz_params.clone()))
                .download()
                .infer_params(InferParams {
                    provider: Provider::OrtCpu,
                    intra_threads: Some(5),
                    ..Default::default()
                })
                .build()?;

        let blaze_face_320_model = FaceDetectorBuilder::new(FaceDetection::BlazeFace320(bz_params))
            .download()
            .infer_params(InferParams {
                provider: Provider::OrtCpu,
                //intra_threads: Some(5),
                ..Default::default()
            })
            .build()?;

        let mtcnn_params = MtCnnParams {
            //thresholds: [0.6, 0.7, 0.7],
            ..MtCnnParams::default()
        };

        let mtcnn_model = FaceDetectorBuilder::new(FaceDetection::MtCnn(mtcnn_params))
            .download()
            .infer_params(InferParams {
                provider: Provider::OrtCpu,
                //intra_threads: Some(5),
                ..Default::default()
            })
            .build()?;

        Ok(FaceExtractor {
            base_path,
            blaze_face_640_model,
            blaze_face_320_model,
            mtcnn_model,
            is_mtcnn_enabled: false,
        })
    }

    /// Identify faces in a photo and return a vector of paths of extracted face images.
    pub async fn extract_faces(
        &self,
        picture_id: &PictureId,
        picture_path: &Path,
    ) -> Result<Vec<Face>> {
        let original_image = Self::open_image(picture_path).await?;

        let image = original_image.clone().into_rgb8().into_array3();

        let mut faces: Vec<(DetectedFace, String)> = vec![];

        let result = self.blaze_face_640_model.detect(image.view().into_dyn());
        if let Ok(detected_faces) = result {
            for f in detected_faces {
                faces.push((f, "blaze_face_640".into()));
            }
        } else {
            error!("Failed extracting faces with back model: {:?}", result);
        }

        let result = self.blaze_face_320_model.detect(image.view().into_dyn());
        if let Ok(detected_faces) = result {
            // Remove any duplicates where being a duplicate is determined by
            // the distance between centres being below a certain threshold

            let detected_faces: Vec<DetectedFace> = detected_faces
                .into_iter()
                .filter(|f1| {
                    let nearest = faces.iter().min_by_key(|f2| {
                        Self::distance(Self::centre(&f1), Self::centre(&f2.0)) as u32
                    });
                    nearest.is_none()
                        || nearest.is_some_and(|f2| {
                            Self::distance(Self::centre(&f1), Self::centre(&f2.0)) > 20.0
                        })
                })
                .collect();

            for f in detected_faces {
                faces.push((f, "blaze_face_320".into()));
            }
        } else {
            error!("Failed extracting faces with front model: {:?}", result);
        }

        if self.is_mtcnn_enabled && faces.is_empty() {
            debug!("BlazeFace models detected zero faces so using slower MTCNN model");
            let result = self.mtcnn_model.detect(image.view().into_dyn());
            if let Ok(detected_faces) = result {
                let detected_faces: Vec<DetectedFace> = detected_faces
                    .into_iter()
                    .filter(|f| f.confidence >= 0.95)
                    .collect();
                for f in detected_faces {
                    faces.push((f, "mtcnn".into()));
                }
            } else {
                error!("Failed extracting faces with MTCNN model: {:?}", result);
            }
        }

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
            .map(|(index, (f, model_name))| {
                if !base_path.exists() {
                    let _ = std::fs::create_dir_all(&base_path);
                }

                // Extract face and save to thumbnail.
                // The bounding box is pretty tight, so make it a bit bigger.
                // Also, make the box a square.

                let longest: u32 =
                    (std::cmp::max(f.rect.width as u32, f.rect.height as u32) as f32 * 1.6) as u32;
                let half_longest: u32 = longest / 2;

                let (centre_x, centre_y) = Self::centre(&f);
                let centre_x = centre_x as u32;
                let centre_y = centre_y as u32;

                // Don't panic when x or y would be < zero
                let x: u32 = centre_x.checked_sub(half_longest).unwrap_or(0);
                let y: u32 = centre_y.checked_sub(half_longest).unwrap_or(0);

                // FIXME use fast_image_resize instead of image-rs
                let thumbnail = original_image.crop_imm(x, y, longest, longest);
                let thumbnail = thumbnail.thumbnail(200, 200);
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
                    model_name,
                }
            })
            .collect();

        // Remove duplicates

        Ok(faces)
    }

    /// Computes Euclidean distance between two points
    fn distance(coord1: (f32, f32), coord2: (f32, f32)) -> f32 {
        let (x1, y1) = coord1;
        let (x2, y2) = coord2;

        let x = x1 - x2;
        let x = x * x;

        let y = y1 - y2;
        let y = y * y;

        f32::sqrt(x + y)
    }

    /// Computes the centre of a face preferring.
    fn centre(f: &DetectedFace) -> (f32, f32) {
        if let Some(ref landmarks) = f.landmarks {
            // If we have landmarks, then the first two are the right and left eyes.
            // Use the midpoint between the eyes as the centre of the thumbnail.
            let x = (landmarks[0].0 + landmarks[1].0) / 2.0;
            let y = (landmarks[0].1 + landmarks[1].1) / 2.0;
            (x, y)
        } else {
            let x = f.rect.x + (f.rect.width / 2.0);
            let y = f.rect.y + (f.rect.height / 2.0);
            (x, y)
        }
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
