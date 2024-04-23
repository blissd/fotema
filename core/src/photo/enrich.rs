// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::thumbnail::Thumbnailer;
use super::Metadata;
use crate::photo::model::{PhotoExtra, PictureId};
use crate::Result;
use std::path::Path;

/// Enrichment operations for photos.
/// Enriches photos with a thumbnail and EXIF metadata.
#[derive(Debug, Clone)]
pub struct Enricher {
    thumbnailer: Thumbnailer,
}

impl Enricher {
    pub fn build(base_path: &Path) -> Result<Enricher> {
        let thumbnailer = Thumbnailer::build(base_path)?;
        Ok(Enricher { thumbnailer })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub async fn enrich(&self, picture_id: &PictureId, picture_path: &Path) -> Result<PhotoExtra> {
        let mut extra = PhotoExtra::default();

        if let Ok(metadata) = Metadata::from_path(picture_path) {
            extra.exif_created_at = metadata.created_at;
            extra.exif_modified_at = metadata.modified_at;
            extra.exif_lens_model = metadata.lens_model;
            extra.content_id = metadata.content_id;
        }

        extra.thumbnail_path = self
            .thumbnailer
            .thumbnail(picture_id, picture_path)
            .await
            .ok();

        Ok(extra)
    }
}
