// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Error::*;
use crate::Result;

use crate::video::model::{ScannedFile, Video, VideoExtra, VideoId};
use chrono::*;
use rusqlite;
use rusqlite::params;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Base path to picture library on file system
    library_base_path: PathBuf,

    /// Base path for thumbnails and transcoded videos
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

        let thumbnail_base_path = PathBuf::from(thumbnail_base_path).join("video_thumbnails");
        let _ = std::fs::create_dir_all(&thumbnail_base_path);

        let repo = Repository {
            library_base_path: PathBuf::from(library_base_path),
            thumbnail_base_path,
            con,
        };

        Ok(repo)
    }

    pub fn update(&mut self, video_id: &VideoId, extra: &VideoExtra) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare(
                    "UPDATE videos
                SET
                    thumbnail_path = ?2,
                    stream_created_ts = ?3,
                    duration_millis = ?4,
                    video_codec = ?5,
                    content_id = ?6,
                    transcoded_path = ?7
                WHERE video_id = ?1",
                )
                .map_err(|e| RepositoryError(e.to_string()))?;

            // convert to relative path before saving to database
            let thumbnail_path = extra
                .thumbnail_path
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.thumbnail_base_path).ok());

            // convert to relative path before saving to database
            let transcoded_path = extra
                .transcoded_path
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.thumbnail_base_path).ok());

            stmt.execute(params![
                video_id.id(),
                thumbnail_path.as_ref().map(|p| p.to_str()),
                extra.stream_created_at,
                extra.stream_duration.map(|x| x.num_milliseconds()),
                extra.video_codec,
                extra.content_id,
                transcoded_path.as_ref().map(|p| p.to_str()),
            ])
            .map_err(|e| RepositoryError(format!("Preview: {}", e)))?;
        }

        tx.commit().map_err(|e| RepositoryError(e.to_string()))
    }

    pub fn add_all(&mut self, vids: &Vec<ScannedFile>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con
            .transaction()
            .map_err(|e| RepositoryError(format!("Starting transaction: {}", e)))?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut vid_stmt = tx
                .prepare_cached(
                    "INSERT INTO videos (
                        video_path,
                        fs_created_ts,
                        link_path
                    ) VALUES (
                        ?1, ?2, ?3
                    ) ON CONFLICT (video_path) DO UPDATE SET
                        fs_created_ts=?2
                    ",
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
                let stem_path = {
                    let file_stem = path
                        .file_stem()
                        .and_then(|x| x.to_str())
                        .expect("Must exist");
                    path.with_file_name(file_stem)
                };

                vid_stmt
                    .execute(params![
                        path.to_str(),
                        vid.fs_created_at,
                        stem_path.to_str(),
                    ])
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
                    thumbnail_path,
                    fs_created_ts,
                    stream_created_ts,
                    duration_millis,
                    video_codec,
                    transcoded_path
                FROM videos
                ORDER BY COALESCE(stream_created_ts, fs_created_ts) ASC",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Video {
                    video_id: VideoId::new(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    thumbnail_path: row
                        .get(2)
                        .ok()
                        .map(|p: String| self.thumbnail_base_path.join(p)),
                    fs_created_at: row.get(3).ok().expect("Must have fs_created_at"),
                    stream_created_at: row.get(4).ok(),
                    stream_duration: row
                        .get(5)
                        .ok()
                        .and_then(|x: i64| TimeDelta::try_milliseconds(x)),
                    video_codec: row.get(6).ok(),
                    transcoded_path: row
                        .get(7)
                        .ok()
                        .map(|p: String| self.thumbnail_base_path.join(p)),
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

        stmt.execute([video_id.id()])
            .map_err(|e| RepositoryError(e.to_string()))?;

        Ok(())
    }
}
