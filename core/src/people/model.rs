// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use crate::thumbnailify;
use chrono::{DateTime, Utc};
use opencv::core::Mat;
use std::fmt::Display;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct FaceDetectionCandidate {
    pub picture_id: PictureId,
    pub host_path: PathBuf,
    pub sandbox_path: PathBuf,
}

impl FaceDetectionCandidate {
    pub fn thumbnail_hash(&self) -> String {
        thumbnailify::compute_hash_for_path(&self.host_path)
    }
}

/// Database ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FaceId(i64);

impl FaceId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// FIXME replace this with a To/From SQL implementation.
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for FaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Person {
    pub person_id: PersonId,
    pub name: String,
    pub thumbnail_path: Option<PathBuf>,
}

/// Database ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonId(i64);

impl PersonId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    /// FIXME replace this with a To/From SQL implementation.
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for PersonId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct Face {
    pub face_id: FaceId,

    /// Path to thumbnail generated from face bounds.
    /// Normalized to be square and expanded to capture the whole head.
    pub thumbnail_path: PathBuf,
}

/// A face hat has been detected, containing the appropriate landmarks to perform
/// a recognition upon the face.
#[derive(Debug, Clone)]
pub struct DetectedFace {
    pub face_id: FaceId,

    /// Path to originally detected face, with no transformations applied
    pub face_path: PathBuf,

    /// When face was detected
    pub detected_at: DateTime<Utc>,

    /// Bounds around face in source image.
    /// NOTE: this is not the same image as is pointed at by face_path.
    pub bounds: Rect,

    /// Landmarks relative to the source image, not relative to the bounds.
    pub right_eye: (f32, f32),
    pub left_eye: (f32, f32),
    pub nose: (f32, f32),
    pub right_mouth_corner: (f32, f32),
    pub left_mouth_corner: (f32, f32),

    pub confidence: f32,
}

impl DetectedFace {
    pub fn landmarks_as_mat(&self) -> Mat {
        // NOTE landmarks are relative to source image, not the bounds, so must translate x and y.
        Mat::from_exact_iter(
            vec![
                0.0,
                0.0,
                self.bounds.width,
                self.bounds.height,
                self.right_eye.0 - self.bounds.x,
                self.right_eye.1 - self.bounds.y,
                self.left_eye.0 - self.bounds.x,
                self.left_eye.1 - self.bounds.y,
                self.nose.0 - self.bounds.x,
                self.nose.1 - self.bounds.y,
                self.right_mouth_corner.0 - self.bounds.x,
                self.right_mouth_corner.1 - self.bounds.y,
                self.left_mouth_corner.0 - self.bounds.x,
                self.left_mouth_corner.1 - self.bounds.y,
                self.confidence,
            ]
            .into_iter(),
        )
        .unwrap()
    }
}

/// A person to perform face recognition for
#[derive(Debug, Clone)]
pub struct PersonForRecognition {
    /// ID of person
    pub person_id: PersonId,

    /// Time of last recognition
    pub recognized_at: DateTime<Utc>,

    /// "Best" confirmed face for person.
    pub face: DetectedFace,
}

/// A face to migrated from Fotema 1.x to Fotema 2.0
#[derive(Debug, Clone)]
pub struct FaceToMigrate {
    pub face_id: FaceId,
    pub face_index: u32,

    /// Path to picture in library.
    /// Relative because people repository cannot have a library base path.
    pub picture_relative_path: PathBuf,
    pub bounds_path: PathBuf,
    pub thumbnail_path: PathBuf,
}

/// A migrated face
#[derive(Debug, Clone)]
pub struct MigratedFace {
    pub face_id: FaceId,
    pub bounds_path: PathBuf,
    pub thumbnail_path: PathBuf,
}
