// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::{PhotoExtra, Picture, PictureId, ScannedFile};

///! Repository of metadata about pictures on the local filesystem.
use crate::Error::*;
use crate::Result;
use rusqlite;
use rusqlite::params;
use rusqlite::Error::SqliteFailure;
use rusqlite::ErrorCode::ConstraintViolation;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Base path to picture library on file system
    library_base_path: PathBuf,

    /// Base path for photo thumbnails
    thumbnail_base_path: PathBuf,

    /// Connection to backing Sqlite database.
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn open(
        library_base_path: &Path,
        thumbnail_base_path: &Path,
        con: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Repository> {
        if !library_base_path.is_dir() {
            return Err(RepositoryError(format!(
                "{:?} is not a directory",
                library_base_path
            )));
        }

        let thumbnail_base_path = PathBuf::from(thumbnail_base_path).join("photo_thumbnails");
        let _ = std::fs::create_dir_all(&thumbnail_base_path);

        let repo = Repository {
            library_base_path: PathBuf::from(library_base_path),
            thumbnail_base_path,
            con,
        };

        Ok(repo)
    }

    pub fn update(&mut self, picture_id: &PictureId, extra: &PhotoExtra) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare(
                    "UPDATE pictures
                SET
                    thumbnail_path = ?2,
                    exif_created_ts = ?3,
                    exif_modified_ts = ?4,
                    is_selfie = ?5,
                    link_date = ?6,
                    content_id = ?7
                WHERE picture_id = ?1",
                )
                .map_err(|e| RepositoryError(e.to_string()))?;

            // convert to relative path before saving to database
            let thumbnail_path = extra
                .thumbnail_path
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.thumbnail_base_path).ok());

            let result = stmt.execute(params![
                picture_id.id(),
                thumbnail_path.as_ref().map(|p| p.to_str()),
                extra.exif_created_at,
                extra.exif_modified_at,
                extra.is_selfie(),
                extra.exif_created_at.map(|x| x.naive_utc().date()),
                extra.content_id,
            ]);

            // The "on conflict ignore" constraints look like errors to rusqlite
            // FIXME get rid of this
            match result {
                Err(e @ SqliteFailure(_, _))
                    if e.sqlite_error_code() == Some(ConstraintViolation) =>
                {
                    // println!("Skipping {:?} {}", path, e);
                }
                other => {
                    other.map_err(|e| RepositoryError(format!("Preview: {}", e)))?;
                }
            }
        }

        tx.commit().map_err(|e| RepositoryError(e.to_string()))
    }

    /// Add all Pictures received from a vector.
    pub fn add_all(&mut self, pics: &Vec<ScannedFile>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(format!("Starting transaction: {}", e)))?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut pic_insert_stmt = tx
                .prepare_cached(
                    "INSERT INTO pictures (
                    picture_path,
                    fs_created_ts,
                    link_path
                ) VALUES (
                    ?1, ?2, $3
                ) ON CONFLICT (picture_path) DO UPDATE SET
                    fs_created_ts=?2
                ",
                )
                .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

            for pic in pics {
                // convert to relative path before saving to database
                let path = pic
                    .path
                    .strip_prefix(&self.library_base_path)
                    .map_err(|e| {
                        RepositoryError(format!("Stripping prefix for {:?}: {}", &pic.path, e))
                    })?;

                // Path without suffix so sibling pictures and videos can be related
                let file_stem = path
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .expect("Must exist");
                let stem_path = path.with_file_name(file_stem);

                pic_insert_stmt
                    .execute(params![
                        path.to_str(),
                        pic.fs_created_at,
                        stem_path.to_str()
                    ])
                    .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;
            }
        }

        tx.commit()
            .map_err(|e| RepositoryError(format!("Committing transaction: {}", e)))
    }

    /// Gets all pictures in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Picture>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con
            .prepare(
                "SELECT
                    pictures.picture_id,
                    pictures.picture_path,
                    pictures.thumbnail_path,
                    pictures.fs_created_ts,
                    pictures.exif_created_ts,
                    pictures.exif_modified_ts,
                    pictures.is_selfie
                FROM pictures
                ORDER BY COALESCE(exif_created_ts, fs_created_ts) ASC",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Picture {
                    picture_id: PictureId::new(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    thumbnail_path: row
                        .get(2)
                        .ok()
                        .map(|p: String| self.thumbnail_base_path.join(p)),
                    fs_created_at: row.get(3).ok().expect("Must have fs_created_ts"),
                    exif_created_at: row.get(4).ok(),
                    exif_modified_at: row.get(5).ok(),
                    is_selfie: row.get(6).ok(),
                })
            })
            .map_err(|e| RepositoryError(e.to_string()))?;

        // Would like to return an iterator... but Rust is defeating me.
        let mut pics = Vec::new();
        for pic in iter.flatten() {
            pics.push(pic);
        }

        Ok(pics)
    }

    pub fn get(&mut self, picture_id: PictureId) -> Result<Option<Picture>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con
            .prepare(
                "SELECT
                    pictures.picture_id,
                    pictures.picture_path,
                    pictures.thumbnail_path,
                    pictures.fs_created_ts,
                    pictures.exif_created_ts,
                    pictures.exif_modified_ts,
                    pictures.is_selfie
                FROM pictures
                WHERE pictures.picture_id = ?1",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([picture_id.id()], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Picture {
                    picture_id: PictureId::new(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    thumbnail_path: row
                        .get(2)
                        .ok()
                        .map(|p: String| self.thumbnail_base_path.join(p)),
                    fs_created_at: row.get(3).ok().expect("Must have fs_created_ts"),
                    exif_created_at: row.get(4).ok(),
                    exif_modified_at: row.get(5).ok(),
                    is_selfie: row.get(6).ok(),
                })
            })
            .map_err(|e| RepositoryError(e.to_string()))?;

        let head = iter.flatten().nth(0);
        Ok(head)
    }

    pub fn remove(&mut self, picture_id: PictureId) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut stmt = con
            .prepare("DELETE FROM pictures WHERE picture_id = ?1")
            .map_err(|e| RepositoryError(e.to_string()))?;

        stmt.execute([picture_id.id()])
            .map_err(|e| RepositoryError(e.to_string()))?;

        Ok(())
    }
}
