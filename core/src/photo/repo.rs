// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::scanner;

///! Repository of metadata about pictures on the local filesystem.
use crate::Error::*;
use crate::Result;
use crate::YearMonth;
use chrono::*;
use rusqlite;
use rusqlite::params;
use rusqlite::Error::SqliteFailure;
use rusqlite::ErrorCode::ConstraintViolation;
use std::fmt::Display;
use std::path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Database ID of picture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureId(i64);

impl PictureId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for PictureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A picture in the repository
#[derive(Debug, Clone)]
pub struct Picture {
    /// Full path from picture library root.
    pub path: PathBuf,

    /// Database primary key for picture
    pub picture_id: PictureId,

    /// Full path to square preview image
    pub square_preview_path: Option<PathBuf>,

    /// Ordering timestamp, derived from EXIF metadata or file system timestamps.
    pub order_by_ts: Option<DateTime<Utc>>,

    /// Was picture taken with front camera?
    pub is_selfie: bool,
}

impl Picture {
    pub fn parent_path(&self) -> Option<PathBuf> {
        self.path.parent().map(|x| PathBuf::from(x))
    }

    pub fn folder_name(&self) -> Option<String> {
        self.path
            .parent()
            .and_then(|x| x.file_name())
            .map(|x| x.to_string_lossy().to_string())
    }

    pub fn year(&self) -> u32 {
        self.order_by_ts
            .map(|ts| ts.date_naive().year_ce().1)
            .unwrap_or(0)
    }

    pub fn year_month(&self) -> YearMonth {
        self.order_by_ts
            .map(|ts| {
                let year = ts.date_naive().year();
                let month = ts.date_naive().month();
                let month = chrono::Month::try_from(u8::try_from(month).unwrap()).unwrap();
                YearMonth { year, month }
            })
            .unwrap_or(YearMonth {
                year: 0,
                month: chrono::Month::January,
            })
    }

    pub fn date(&self) -> Option<chrono::NaiveDate> {
        self.order_by_ts.map(|ts| ts.date_naive())
    }
}

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Base path to picture library on file system
    library_base_path: path::PathBuf,

    photo_thumbnail_base_path: path::PathBuf,

    /// Connection to backing Sqlite database.
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn open(
        library_base_path: &path::Path,
        photo_thumbnail_base_path: &path::Path,
        con: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Repository> {
        if !library_base_path.is_dir() {
            return Err(RepositoryError(format!(
                "{:?} is not a directory",
                library_base_path
            )));
        }

        let repo = Repository {
            library_base_path: path::PathBuf::from(library_base_path),
            photo_thumbnail_base_path: path::PathBuf::from(photo_thumbnail_base_path),
            con,
        };

        Ok(repo)
    }

    pub fn add_preview(&mut self, pic: &Picture) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare("UPDATE pictures SET preview_path = ?1 WHERE picture_id = ?2")
                .map_err(|e| RepositoryError(e.to_string()))?;

            // convert to relative path before saving to database
            let thumbnail_path = pic
                .square_preview_path
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.photo_thumbnail_base_path).ok());

            let result = stmt.execute(params![
                thumbnail_path.as_ref().map(|p| p.to_str()),
                pic.picture_id.0,
            ]);

            //result
            //  .map(|_| ())
            //.map_err(|e| RepositoryError(e.to_string()))?;

            // The "on conflict ignore" constraints look like errors to rusqlite
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
    pub fn add_all(&mut self, pics: &Vec<scanner::Picture>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(format!("Starting transaction: {}", e)))?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut pic_lookup_stmt = tx
                .prepare_cached(
                    "SELECT picture_id
                    FROM pictures
                    WHERE picture_path = ?1",
                )
                .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

            let mut pic_insert_stmt = tx
                .prepare_cached(
                    "INSERT INTO pictures (
                    picture_path,
                    order_by_ts,
                    is_selfie
                ) VALUES (
                    ?1, ?2, ?3
                ) ON CONFLICT (picture_path) DO UPDATE SET order_by_ts=?2, is_selfie=?3",
                )
                .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

            let mut vis_insert_stmt = tx
                .prepare_cached(
                    "INSERT INTO visual (
                        stem_path,
                        picture_id
                    ) VALUES (
                        ?1,
                        ?2
                    ) ON CONFLICT (stem_path) DO UPDATE SET picture_id = ?2",
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

                let exif_date_time = pic.exif.as_ref().and_then(|x| x.created_at);
                let fs_date_time = pic.fs.as_ref().and_then(|x| x.created_at);
                let order_by_ts = exif_date_time.map(|d| d.to_utc()).or(fs_date_time);
                let is_selfie = pic
                    .exif
                    .as_ref()
                    .and_then(|x| x.lens_model.as_ref())
                    .is_some_and(|x| x.contains("front"));

                // Path without suffix so sibling pictures and videos can be related
                let file_stem = path
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .expect("Must exist");
                let stem_path = path.with_file_name(file_stem);

                pic_insert_stmt
                    .execute(params![path.to_str(), order_by_ts, is_selfie])
                    .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

                let picture_id = pic_lookup_stmt
                    .query_row(params![path.to_str()], |row| {
                        let id: i64 = row.get(0).expect("Must have picture_id");
                        Ok(id)
                    })
                    .map_err(|e| RepositoryError(format!("Must have picture_id: {}", e)))?;

                vis_insert_stmt
                    .execute(params![stem_path.to_str(), picture_id])
                    .map_err(|e| RepositoryError(format!("Inserting: {}", e)))?;
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
                    pictures.preview_path as square_preview_path,
                    pictures.order_by_ts,
                    pictures.is_selfie
                FROM pictures
                ORDER BY order_by_ts ASC",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Picture {
                    picture_id: PictureId(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    square_preview_path: row
                        .get(2)
                        .ok()
                        .map(|p: String| self.photo_thumbnail_base_path.join(p)),
                    order_by_ts: row.get(3).ok(),
                    is_selfie: row.get(4).ok().unwrap_or(false),
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
                    pictures.preview_path as square_preview_path,
                    pictures.order_by_ts,
                    pictures.is_selfie
                FROM pictures
                WHERE pictures.picture_id = ?1",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([picture_id.0], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Picture {
                    picture_id: PictureId(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    square_preview_path: row
                        .get(2)
                        .ok()
                        .map(|p: String| self.photo_thumbnail_base_path.join(p)),
                    order_by_ts: row.get(3).ok(),
                    is_selfie: row.get(4).ok().unwrap_or(false),
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

        stmt.execute([picture_id.0])
            .map_err(|e| RepositoryError(e.to_string()))?;

        Ok(())
    }
}
