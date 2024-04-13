// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::video::scanner;
use crate::Error::*;
use crate::Result;

use chrono::*;
use rusqlite;
use rusqlite::params;
use rusqlite::Error::SqliteFailure;
use rusqlite::ErrorCode::ConstraintViolation;
use std::fmt::Display;
use std::path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Database ID of video
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoId(i64);

impl VideoId {
    pub fn new(id: i64) -> Self {
        Self(id)
    }

    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for VideoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A video in the repository
#[derive(Debug, Clone)]
pub struct Video {
    /// Full path from library root.
    pub path: PathBuf,

    /// Database primary key for video
    pub video_id: VideoId,

    /// Full path to square preview image
    pub thumbnail_path: Option<PathBuf>,

    /// Filesystem creation timestamp
    pub fs_created_at: DateTime<Utc>,
}

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Base path to picture library on file system
    library_base_path: path::PathBuf,

    video_thumbnail_base_path: path::PathBuf,

    /// Connection to backing Sqlite database.
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn open(
        library_base_path: &path::Path,
        video_thumbnail_base_path: &path::Path,
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
            video_thumbnail_base_path: path::PathBuf::from(video_thumbnail_base_path),
            con,
        };

        Ok(repo)
    }

    pub fn update(&mut self, pic: &Video) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare("UPDATE videos SET preview_path = ?1 WHERE video_id = ?2")
                .map_err(|e| RepositoryError(e.to_string()))?;

            // convert to relative path before saving to database
            let thumbnail_path = pic
                .thumbnail_path
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.video_thumbnail_base_path).ok());

            let result = stmt.execute(params![
                thumbnail_path.as_ref().map(|p| p.to_str()),
                pic.video_id.0,
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

    pub fn add_all(&mut self, vids: &Vec<scanner::Video>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(format!("Starting transaction: {}", e)))?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut vid_lookup_stmt = tx
                .prepare_cached(
                    "SELECT video_id
                FROM videos
                WHERE video_path = ?1",
                )
                .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

            let mut vid_stmt = tx
                .prepare_cached(
                    "INSERT INTO videos (
                        video_path,
                        fs_created_ts
                    ) VALUES (
                        ?1, ?2
                    ) ON CONFLICT (video_path) DO UPDATE SET fs_created_ts=?2
                    ",
                )
                .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

            let mut vis_stmt = tx
                .prepare_cached(
                    "INSERT INTO visual (
                        stem_path,
                        video_id
                    ) VALUES (
                        ?1,
                        ?2
                    ) ON CONFLICT (stem_path) DO UPDATE SET video_id = ?2",
                )
                .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

            for vid in vids {
                // convert to relative path before saving to database
                let path = vid
                    .path
                    .strip_prefix(&self.library_base_path)
                    .map_err(|e| {
                        RepositoryError(format!("Stripping prefix for {:?}: {}", &vid.path, e))
                    })?;

                // Path without suffix so sibling pictures and videos can be related
                let file_stem = path
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .expect("Must exist");
                let stem_path = path.with_file_name(file_stem);

                let fs_created_at = vid.fs.as_ref().and_then(|x| x.created_at);
                if fs_created_at.is_none() {
                    continue; // FIXME scanner::Video should not be an Option
                }

                let fs_created_at = fs_created_at.expect("Must have fs_created_at");

                vid_stmt
                    .execute(params![path.to_str(), fs_created_at])
                    .map_err(|e| RepositoryError(format!("Inserting: {}", e)))?;

                let video_id = vid_lookup_stmt
                    .query_row(params![path.to_str()], |row| {
                        let id: i64 = row.get(0).expect("Must have video_id");
                        Ok(id)
                    })
                    .map_err(|e| RepositoryError(format!("Must have video_id: {}", e)))?;

                vis_stmt
                    .execute(params![stem_path.to_str(), video_id])
                    .map_err(|e| RepositoryError(format!("Inserting: {}", e)))?;
            }
        }

        tx.commit()
            .map_err(|e| RepositoryError(format!("Committing transaction: {}", e)))
    }

    /// Gets all videos in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Video>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con
            .prepare(
                "SELECT
                    video_id,
                    video_path,
                    preview_path,
                    fs_created_ts,
                FROM videos
                ORDER BY created_ts ASC",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Video {
                    video_id: VideoId(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    thumbnail_path: row
                        .get(2)
                        .ok()
                        .map(|p: String| self.video_thumbnail_base_path.join(p)),
                    fs_created_at: row.get(3).ok().expect("Must have fs_created_at"),
                })
            })
            .map_err(|e| RepositoryError(e.to_string()))?;

        // Would like to return an iterator... but Rust is defeating me.
        let mut vids = Vec::new();
        for vid in iter.flatten() {
            vids.push(vid);
        }

        Ok(vids)
    }

    pub fn remove(&mut self, video_id: VideoId) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut stmt = con
            .prepare("DELETE FROM videos WHERE video_id = ?1")
            .map_err(|e| RepositoryError(e.to_string()))?;

        stmt.execute([video_id.0])
            .map_err(|e| RepositoryError(e.to_string()))?;

        Ok(())
    }
}
