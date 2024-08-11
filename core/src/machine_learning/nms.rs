// SPDX-FileCopyrightText: © 2024 David Bliss
// SPDX-FileCopyrightText: © 2023 Rusty Builder Indies
//
// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use rust_faces::Face;

/// Non-maximum suppression.
#[derive(Copy, Clone, Debug)]
pub struct Nms {
    pub iou_threshold: f32,
}

impl Default for Nms {
    fn default() -> Self {
        Self { iou_threshold: 0.3 }
    }
}

impl Nms {
    /// Suppress non-maxima faces.
    ///
    /// # Arguments
    ///
    /// * `faces` - Faces to suppress.
    ///
    /// # Returns
    ///
    /// * `Vec<Face>` - Suppressed faces.
    ///
    /// This method is lifted from the rust-faces project and modified to add turn
    /// the face into a tuple that carries a model name.
    pub fn suppress_non_maxima(&self, mut faces: Vec<(Face, String)>) -> Vec<(Face, String)> {
        faces.sort_by(|a, b| a.0.confidence.partial_cmp(&b.0.confidence).unwrap());

        let mut faces_map = HashMap::new();
        faces.iter().rev().enumerate().for_each(|(i, face)| {
            faces_map.insert(i, face);
        });

        let mut nms_faces = Vec::with_capacity(faces.len());
        let mut count = 0;
        while !faces_map.is_empty() {
            if let Some((_, face)) = faces_map.remove_entry(&count) {
                nms_faces.push(face.clone());
                faces_map.retain(|_, face2| face.0.rect.iou(&face2.0.rect) < self.iou_threshold);
            }
            count += 1;
        }

        nms_faces
    }
}
