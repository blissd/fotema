// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Repository;
use anyhow::*;
use tracing::{error, info};

use super::model::MigratedFace;
use crate::FlatpakPathBuf;
use crate::thumbnailify;

use std::path::{Path, PathBuf};

/// Migrate face/people data from Fotema 1.0 to Fotema 2.0.
/// This file should be deleted in a year or so when we are confident most users
/// are on Fotema 2.0+.
#[derive(Debug, Clone)]
pub struct Migrate {
    data_dir_base_path: PathBuf,
    library_dir_base_path: FlatpakPathBuf,
    repo: Repository,
}

impl Migrate {
    pub fn build(
        repo: Repository,
        data_dir_base_path: &Path,
        library_dir_base_path: FlatpakPathBuf,
    ) -> Migrate {
        Migrate {
            repo,
            data_dir_base_path: data_dir_base_path.into(),
            library_dir_base_path,
        }
    }

    pub fn migrate(&mut self) -> Result<()> {
        // Delete face scans for pictures that do _not_ have a confirmed face.
        // This will cause Fotema to re-scan all the previously scanned pictures, except those
        // that have a confirmed face.
        let faces_to_migrate = self.repo.migrate_get_all()?;

        if faces_to_migrate.is_empty() {
            info!("No faces to migrate");
            return Ok(());
        }

        let base_dir = self.data_dir_base_path.join("faces");
        std::fs::create_dir_all(&base_dir)?;

        info!("Migrating {} faces", faces_to_migrate.len());

        faces_to_migrate.into_iter().for_each(|f| {
            let picture_path = self
                .library_dir_base_path
                .host_path
                .join(f.picture_relative_path);
            info!(
                "Migrating face detection and recognition files for {:?}",
                picture_path
            );

            let file_uri = thumbnailify::get_file_uri(&picture_path).unwrap();
            let file_uri_hash = thumbnailify::compute_hash(&file_uri);

            let thumbnail_path =
                base_dir.join(format!("{}_{}_thumbnail.png", file_uri_hash, f.face_index));

            let bounds_path =
                base_dir.join(format!("{}_{}_original.png", file_uri_hash, f.face_index));

            let _ = std::fs::rename(&f.thumbnail_path, &thumbnail_path)
                .and_then(|()| std::fs::rename(&f.bounds_path, &bounds_path))
                .map_err(|e| {
                    error!("Failed migrating face {:?}: {:?}", f.face_id, e);
                    e
                });

            let bounds_path = bounds_path
                .strip_prefix(&self.data_dir_base_path)
                .expect("Must strip");

            let thumbnail_path = thumbnail_path
                .strip_prefix(&self.data_dir_base_path)
                .expect("Must strip");

            let mf = MigratedFace {
                face_id: f.face_id,
                bounds_path: bounds_path.into(),
                thumbnail_path: thumbnail_path.into(),
            };

            let _ = self.repo.migrate_update_face_paths(mf).map_err(|e| {
                error!("Failed migrating face {:?}: {:?}", f.face_id, e);
                e
            });
        });

        let dir_to_delete = self.data_dir_base_path.join("photo_faces");
        let _ = std::fs::remove_dir_all(&dir_to_delete).map_err(|e| {
            error!("Failed to delete {:?}: {:?}", dir_to_delete, e);
            e
        });

        let _ = self.repo.migrate_truncate().map_err(|e| {
            error!("Failed to truncate: {:?}", e);
            e
        });

        Ok(())
    }
}
