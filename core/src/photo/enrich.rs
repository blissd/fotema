// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use crate::photo::model::{PhotoExtra, PictureId};
use crate::Error::*;
use crate::Result;
use gdk4::prelude::TextureExt;
use glycin;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use std::path::{Path, PathBuf};
use tempfile;

const EDGE: u32 = 200;

/// Enrichment operations for photos.
/// Enriches photos with a thumbnail and EXIF metadata.
#[derive(Debug, Clone)]
pub struct Enricher {
    base_path: PathBuf,
}

impl Enricher {
    pub fn build(base_path: &Path) -> Result<Enricher> {
        let base_path = PathBuf::from(base_path);
        std::fs::create_dir_all(base_path.join("square"))
            .map_err(|e| RepositoryError(e.to_string()))?;
        Ok(Enricher { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub async fn enrich(&self, picture_id: &PictureId, picture_path: &Path) -> Result<PhotoExtra> {
        let mut extra = PhotoExtra::default();

        let thumbnail_path = {
            let file_name = format!("{}_{}x{}.png", picture_id, EDGE, EDGE);
            self.base_path.join(file_name)
        };

        self.compute_thumbnail(picture_path, &thumbnail_path)
            .await
            .map_err(|e| PreviewError(format!("save photo thumbnail: {}", e)))?;

        extra.thumbnail_path = Some(thumbnail_path.clone());

        if let Ok(metadata) = Metadata::from_path(picture_path) {
            extra.exif_created_at = metadata.created_at;
            extra.exif_modified_at = metadata.modified_at;
            extra.exif_lens_model = metadata.lens_model;
        }

        Ok(extra)
    }
    async fn compute_thumbnail(&self, picture_path: &Path, thumbnail_path: &Path) -> Result<()> {
        if thumbnail_path.exists() {
            return Ok(());
        }

        let thumbnail = self.standard_thumbnail(picture_path);

        let thumbnail = if thumbnail.is_err() {
            self.fallback_thumbnail(picture_path).await
        } else {
            thumbnail
        }?;

        thumbnail
            .save(thumbnail_path)
            .or_else(|e| {
                let _ = std::fs::remove_file(&thumbnail_path);
                Err(e) // don't lose original error
            })
            .map_err(|e| PreviewError(format!("image save: {}", e)))?;

        Ok(())
    }

    fn standard_thumbnail(&self, path: &Path) -> Result<DynamicImage> {
        let img = ImageReader::open(path)
            .map_err(|e| PreviewError(format!("image open: {}", e)))?
            .decode()
            .map_err(|e| PreviewError(format!("image decode: {}", e)))?;

        let img = if img.width() == img.height() && img.width() == EDGE {
            return Ok(img);
        } else if img.width() == img.height() {
            img
        } else if img.width() < img.height() {
            let h = (img.height() - img.width()) / 2;
            img.crop_imm(0, h, img.width(), img.width())
        } else {
            let w = (img.width() - img.height()) / 2;
            img.crop_imm(w, 0, img.height(), img.height())
        };

        let img = img.thumbnail(EDGE, EDGE);
        Ok(img)
    }

    /// Copy an image to a PNG file using Glycin, and then use image-rs to compute the thumbnail.
    /// This is the fallback if image-rs can't decode the original image (such as HEIC images).
    async fn fallback_thumbnail(&self, path: &Path) -> Result<DynamicImage> {
        let file = gio::File::for_path(path);

        let image = glycin::Loader::new(file)
            .load()
            .await
            .map_err(|e| PreviewError(format!("Glycin load image: {}", e)))?;

        let frame = image
            .next_frame()
            .await
            .map_err(|e| PreviewError(format!("Glycin image frame: {}", e)))?;

        let png_file = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(|e| PreviewError(format!("Temp file: {}", e)))?;

        frame
            .texture
            .save_to_png(png_file.path())
            .map_err(|e| PreviewError(format!("image save: {}", e)))?;

        self.standard_thumbnail(png_file.path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn picture_dir() -> PathBuf {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        test_data_dir
    }

    #[test]
    fn test_to_square() {
        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Frog.jpg");

        let target_dir = PathBuf::from("target");
        let prev = Previewer::build(&target_dir).unwrap();
        let img = prev.from_path(&test_file).unwrap();
        let output = target_dir.join("out.jpg");
        let _ = img.save(&output);
    }
}
