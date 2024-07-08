// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use std::path::{Path, PathBuf};

use candle_nn::{Module, VarBuilder};
use candle_transformers::models::mobileone;

use crate::face::blaze_face;

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

#[derive(Debug, Clone)]
pub struct FaceExtractor {
    base_path: PathBuf,
}

impl FaceExtractor {
    pub fn build(base_path: &Path) -> Result<FaceExtractor> {
        let base_path = PathBuf::from(base_path).join("photo_faces");
        std::fs::create_dir_all(&base_path)?;

        Ok(FaceExtractor { base_path })
    }

    /// Identify faces in a photo and return a vector of paths of extracted face images.
    pub fn extract_faces(&self, picture_id: &PictureId, picture_path: &Path) -> Result<Vec<Face>> {
        todo!("do this")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_faces() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let file = Path::new(dir).join("resources/test/Sandow.jpg");

        let cache_dir = Path::new(".");
    }
}
