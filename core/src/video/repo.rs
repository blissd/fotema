// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::metadata;
use super::Metadata;
use crate::video::model::{ScannedFile, Video, VideoId};
use anyhow::*;
use chrono::*;
use rusqlite;
use rusqlite::params;
use rusqlite::Row;
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
        let thumbnail_base_path = PathBuf::from(thumbnail_base_path);
        std::fs::create_dir_all(&thumbnail_base_path)?;

        let repo = Repository {
            library_base_path: PathBuf::from(library_base_path),
            thumbnail_base_path,
            con,
        };

        Ok(repo)
    }

    pub fn add_thumbnail(&mut self, video_id: &VideoId, thumbnail_path: &Path) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare(
                "UPDATE videos
                SET
                    thumbnail_path = ?2
                WHERE video_id = ?1",
            )?;

            // convert to relative path before saving to database
            let thumbnail_path = thumbnail_path.strip_prefix(&self.thumbnail_base_path).ok();

            stmt.execute(params![
                video_id.id(),
                thumbnail_path.as_ref().map(|p| p.to_str()),
            ])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn add_metadata(&mut self, vids: Vec<(VideoId, Metadata)>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare(
                "UPDATE videos
                SET
                    metadata_version = ?2,
                    stream_created_ts = ?3,
                    duration_millis = ?4,
                    video_codec = ?5,
                    content_id = ?6
                WHERE video_id = ?1",
            )?;

            for (video_id, metadata) in vids {
                stmt.execute(params![
                    video_id.id(),
                    metadata::VERSION,
                    metadata.created_at,
                    metadata.duration.map(|x| x.num_milliseconds()),
                    metadata.video_codec,
                    metadata.content_id,
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn add_all(&mut self, vids: &Vec<ScannedFile>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut vid_stmt = tx.prepare_cached(
                "INSERT INTO videos (
                        video_path,
                        fs_created_ts,
                        link_path
                    ) VALUES (
                        ?1, ?2, ?3
                    ) ON CONFLICT (video_path) DO UPDATE SET
                        fs_created_ts=?2
                    ",
            )?;

            for vid in vids {
                // convert to relative path before saving to database
                let path = vid.path.strip_prefix(&self.library_base_path)?;

                // Path without suffix so sibling pictures and videos can be related
                let stem_path = {
                    let file_stem = path
                        .file_stem()
                        .and_then(|x| x.to_str())
                        .expect("Must exist");
                    path.with_file_name(file_stem)
                };

                vid_stmt.execute(params![
                    path.to_str(),
                    vid.fs_created_at,
                    stem_path.to_str(),
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Gets all videos in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Video>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
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
        )?;

        let result = stmt.query_map([], |row| self.to_video(row))?;
        let result = result.flatten().collect();
        Ok(result)
    }

    /// Gets all videos in the repository, in ascending order of modification timestamp.
    pub fn find_need_metadata_update(&self) -> Result<Vec<Video>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
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
                WHERE metadata_version < ?1
                ORDER BY COALESCE(stream_created_ts, fs_created_ts) ASC",
        )?;

        let result = stmt.query_map([metadata::VERSION], |row| self.to_video(row))?;
        let result = result.flatten().collect();
        Ok(result)
    }

    fn to_video(&self, row: &Row<'_>) -> rusqlite::Result<Video> {
        let video_id = row.get("video_id").map(|x| VideoId::new(x))?;

        let video_path: String = row.get("video_path")?;
        let video_path = self.library_base_path.join(video_path);

        let thumbnail_path = row
            .get("thumbnail_path")
            .map(|p: String| self.thumbnail_base_path.join(p))
            .ok();

        let fs_created_at = row.get("fs_created_ts")?;
        let stream_created_at = row.get("stream_created_at").ok();

        let stream_duration = row
            .get("stream_duration")
            .ok()
            .and_then(|x: i64| TimeDelta::try_milliseconds(x));

        let video_codec = row.get("video_codec").ok();

        let transcoded_path = row
            .get("transcoded_path")
            .map(|p: String| self.thumbnail_base_path.join(p))
            .ok();

        std::result::Result::Ok(Video {
            video_id,
            path: video_path,
            thumbnail_path,
            fs_created_at,
            stream_created_at,
            stream_duration,
            video_codec,
            transcoded_path,
        })
    }

    pub fn remove(&mut self, video_id: VideoId) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare("DELETE FROM videos WHERE video_id = ?1")?;

        stmt.execute([video_id.id()])?;

        Ok(())
    }
}
