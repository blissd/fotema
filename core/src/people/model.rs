// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use opencv::core::Mat;
use std::fmt::Display;
use std::path::PathBuf;

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
    pub thumbnail_path: PathBuf,
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
pub struct DetectedFace {
    pub face_id: FaceId,

    /// Path to originally detected face, with no transformations applied
    pub face_path: PathBuf,

    /// Bounds around face in source image.
    /// NOTE: this is not the same image as is pointed at by face_path.
    bounds: Rect,

    /// Landmarks relative to the source image, not relative to the bounds.
    right_eye: (f32, f32),
    left_eye: (f32, f32),
    nose: (f32, f32),
    right_mouth_corner: (f32, f32),
    left_mouth_corner: (f32, f32),

    confidence: f32,
}

impl DetectedFace {
    pub fn landmarks_as_mat(&self) -> Mat {
        Mat::from_slice_2d(&[[
            self.bounds.x,
            self.bounds.y,
            self.bounds.width,
            self.bounds.height,
            self.right_eye.0,
            self.right_eye.1,
            self.left_eye.0,
            self.left_eye.1,
            self.nose.0,
            self.nose.1,
            self.right_mouth_corner.0,
            self.right_mouth_corner.1,
            self.left_mouth_corner.0,
            self.left_mouth_corner.1,
            self.confidence,
        ]])
        .unwrap()
    }
}
