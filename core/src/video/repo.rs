// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use super::metadata;
use crate::FlatpakPathBuf;
use crate::ScannedFile;
use crate::file_types;
use crate::path_encoding;
use crate::video::model::{Video, VideoId};

use anyhow::{Result, bail};
use chrono::*;
use rusqlite::{self, OptionalExtension, Row, params};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::error;

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Base paths to library
    library_base_dirs: Vec<FlatpakPathBuf>,

    /// Cached mapping of (root_id, FlatpakPathBuf)
    roots: Vec<(i64, FlatpakPathBuf)>,

    /// Base path for transcoded videos
    cache_dir_base_path: PathBuf,

    /// Base path for data directory
    data_dir_base_path: PathBuf,

    /// Connection to backing Sqlite database.
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn open(
        library_base_dir: &FlatpakPathBuf,
        cache_dir_base_path: &Path,
        data_dir_base_path: &Path,
        con: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Repository> {
        Self::open_multiple(
            &[library_base_dir.clone()],
            cache_dir_base_path,
            data_dir_base_path,
            con,
        )
    }

    pub fn open_multiple(
        library_base_dirs: &[FlatpakPathBuf],
        cache_dir_base_path: &Path,
        data_dir_base_path: &Path,
        con: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Repository> {
        std::fs::create_dir_all(cache_dir_base_path)?;

        let mut repo = Repository {
            library_base_dirs: library_base_dirs.to_vec(),
            roots: Vec::new(),
            cache_dir_base_path: cache_dir_base_path.into(),
            data_dir_base_path: data_dir_base_path.into(),
            con,
        };
        repo.sync_roots()?;
        Ok(repo)
    }

    pub fn sync_roots(&mut self) -> Result<()> {
        let con = self.con.lock().unwrap();
        if let Some(first) = self.library_base_dirs.first() {
            let first_host_b64 = path_encoding::to_base64(&first.host_path);
            let existing_first_root: Option<i64> = con
                .query_row(
                    "SELECT root_id FROM library_roots WHERE host_path_b64 = ?1",
                    params![first_host_b64],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(existing_id) = existing_first_root {
                if existing_id != 1 {
                    con.execute(
                        "UPDATE OR IGNORE videos SET root_id = ?1 WHERE root_id = 1",
                        params![existing_id],
                    )?;
                    con.execute(
                        "DELETE FROM library_roots WHERE root_id = 1 AND host_path_b64 = ''",
                        [],
                    )?;
                }
            } else {
                con.execute(
                    "UPDATE library_roots SET host_path_b64 = ?1 WHERE root_id = 1 AND host_path_b64 = ''",
                    params![first_host_b64],
                )?;
            }
        }

        for dir in &self.library_base_dirs {
            let host_b64 = path_encoding::to_base64(&dir.host_path);
            con.execute(
                "INSERT INTO library_roots (host_path_b64) VALUES (?1) ON CONFLICT (host_path_b64) DO NOTHING",
                params![host_b64],
            )?;
        }

        self.roots = self
            .library_base_dirs
            .iter()
            .map(|dir| {
                let host_b64 = path_encoding::to_base64(&dir.host_path);
                let root_id: Option<i64> = con
                    .query_row(
                        "SELECT root_id FROM library_roots WHERE host_path_b64 = ?1",
                        params![host_b64],
                        |row| row.get(0),
                    )
                    .optional()
                    .ok()
                    .flatten();
                (root_id.unwrap_or(1), dir.clone())
            })
            .collect();

        Ok(())
    }

    pub fn clean_removed_roots(&mut self, active_dirs: &[FlatpakPathBuf]) -> Result<()> {
        let active_b64s: Vec<String> = active_dirs
            .iter()
            .map(|d| path_encoding::to_base64(&d.host_path))
            .collect();
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare("SELECT root_id, host_path_b64 FROM library_roots")?;
        let roots: Vec<(i64, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .flatten()
            .collect();
        drop(stmt);
        drop(con);

        for (root_id, host_b64) in roots {
            if !active_b64s.contains(&host_b64) && !host_b64.is_empty() {
                self.remove_root(root_id)?;
            }
        }
        self.library_base_dirs = active_dirs.to_vec();
        self.roots.retain(|(_, dir)| active_dirs.contains(dir));
        Ok(())
    }

    pub fn remove_root(&mut self, root_id: i64) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut vid_stmt = con.prepare("SELECT video_id FROM videos WHERE root_id = ?1")?;
        let vid_ids: Vec<VideoId> = vid_stmt
            .query_map([root_id], |row| Ok(VideoId::new(row.get(0)?)))?
            .flatten()
            .collect();
        drop(vid_stmt);
        drop(con);

        for vid_id in vid_ids {
            if let Ok(files) = self.find_files_to_cleanup(vid_id) {
                for file in files {
                    let _ = std::fs::remove_file(file);
                }
            }
        }

        let con = self.con.lock().unwrap();
        con.execute("DELETE FROM videos WHERE root_id = ?1", [root_id])?;
        con.execute("DELETE FROM library_roots WHERE root_id = ?1", [root_id])?;
        Ok(())
    }

    fn find_root_for_sandbox_path(&self, path: &Path) -> Result<(i64, FlatpakPathBuf)> {
        for (root_id, dir) in &self.roots {
            if path.starts_with(&dir.sandbox_path) {
                return Ok((*root_id, dir.clone()));
            }
        }
        if let Some((root_id, first)) = self.roots.first() {
            Ok((*root_id, first.clone()))
        } else if let Some(first) = self.library_base_dirs.first() {
            Ok((1, first.clone()))
        } else {
            bail!("No library base directories configured");
        }
    }

    fn get_root_by_id(&self, root_id: i64) -> FlatpakPathBuf {
        for (id, dir) in &self.roots {
            if *id == root_id {
                return dir.clone();
            }
        }
        self.roots
            .first()
            .map(|(_, d)| d.clone())
            .or_else(|| self.library_base_dirs.first().cloned())
            .unwrap_or_default()
    }

    pub fn mark_broken(&mut self, video_id: &VideoId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare(
                "UPDATE videos
                SET
                    is_broken = TRUE
                WHERE video_id = ?1",
            )?;

            stmt.execute(params![video_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn add_transcode(&mut self, video_id: VideoId, transcoded_path: &Path) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare(
                "UPDATE videos
                SET
                    transcoded_path = ?2
                WHERE video_id = ?1",
            )?;

            // convert to relative path before saving to database
            let transcoded_path = transcoded_path.strip_prefix(&self.cache_dir_base_path).ok();

            stmt.execute(params![
                video_id.id(),
                transcoded_path.as_ref().map(|p| p.to_str()),
            ])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn add_metadata(&mut self, vids: Vec<(VideoId, Metadata)>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut update_videos = tx.prepare(
                "UPDATE videos
                SET
                    metadata_version = ?2,
                    stream_created_ts = ?3,
                    duration_millis = ?4,
                    video_codec = ?5,
                    content_id = ?6,
                    rotation = ?7,
                    fs_created_ts = ?8,
                    fs_modified_ts = ?9
                WHERE video_id = ?1",
            )?;

            let mut update_geo = tx.prepare_cached(
                "INSERT INTO videos_geo (
                    video_id,
                    latitude,
                    longitude
                ) VALUES (
                    ?1, ?2, ?3
                ) ON CONFLICT (video_id) DO UPDATE SET
                    latitude = ?2,
                    longitude = ?3
                ",
            )?;

            for (video_id, metadata) in vids {
                update_videos.execute(params![
                    video_id.id(),
                    metadata::VERSION,
                    metadata.stream_created_at,
                    metadata.duration.map(|x| x.num_milliseconds()),
                    metadata.video_codec,
                    metadata.content_id,
                    metadata.rotation,
                    metadata.fs_created_at,
                    metadata.fs_modified_at,
                ])?;

                if let Some(location) = metadata.location {
                    // Belts and braces.
                    // SQLite will treat a "nan" (not-a-number) as a null and cause
                    // the not-null constraint to be violated.
                    let latitude = location.latitude.to_f64_safe();
                    let longitude = location.longitude.to_f64_safe();
                    if latitude.is_some() && longitude.is_some() {
                        update_geo.execute(params![video_id.id(), latitude, longitude,])?;
                    }
                }
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
                        root_id,
                        video_path_b64,
                        video_path_lossy,
                        link_path_b64,
                        link_path_lossy,
                        insert_ts
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP
                    ) ON CONFLICT(root_id, video_path_b64) DO UPDATE SET
                        link_path_b64 = ?4,
                        link_path_lossy = ?5",
            )?;

            for scanned_file in vids {
                if let ScannedFile::Video(path) = scanned_file {
                    if let Ok((root_id, root)) = self.find_root_for_sandbox_path(path) {
                        // convert to relative path before saving to database
                        if let Ok(video_path) = path.strip_prefix(&root.sandbox_path) {
                            let video_path_b64 = path_encoding::to_base64(video_path);

                            // Path without suffix so sibling pictures and videos can be related
                            // Some Apple photo exports name the video component after the
                            // picture component, so you get names like 'img_1234.heic.mp4' instead
                            // of 'img_1234.mp4'. After removing the file suffix, check if a supported
                            // picture suffix is now present and remove it.
                            let link_path = match video_path.file_stem() {
                                Some(stem) if file_types::is_supported_picture(&stem.as_ref()) => {
                                    let path: &Path = stem.as_ref();
                                    path.file_stem()
                                }
                                any => any,
                            };

                            let link_path = link_path.and_then(|x| x.to_str()).expect("Must exist");

                            let link_path = video_path.with_file_name(link_path);
                            let link_path_b64 = path_encoding::to_base64(&link_path);

                            vid_stmt.execute(params![
                                root_id,
                                video_path_b64,
                                video_path.to_string_lossy(),
                                link_path_b64,
                                link_path.to_string_lossy(),
                            ])?;
                        }
                    }
                } else {
                    error!("Expected a video, but got: {:?}", scanned_file);
                }
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
                    root_id,
                    video_path_b64,
                    COALESCE(
                        videos.stream_created_ts,
                        videos.fs_created_ts,
                        videos.fs_modified_ts,
                        videos.insert_ts
                    ) AS ordering_ts,
                    duration_millis,
                    video_codec,
                    transcoded_path
                FROM videos
                WHERE COALESCE(is_broken, FALSE) IS FALSE
                ORDER BY ordering_ts ASC",
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
                    root_id,
                    video_path_b64,
                    COALESCE(
                        videos.stream_created_ts,
                        videos.fs_created_ts,
                        videos.fs_modified_ts,
                        videos.insert_ts
                    ) AS ordering_ts,
                    duration_millis,
                    video_codec,
                    transcoded_path
                FROM videos
                WHERE metadata_version < ?1
                AND COALESCE(is_broken, FALSE) IS FALSE
                ORDER BY ordering_ts ASC",
        )?;

        let result = stmt.query_map([metadata::VERSION], |row| self.to_video(row))?;
        let result = result.flatten().collect();
        Ok(result)
    }

    /// Gets paths of files to delete when a video is no longer present.
    pub fn find_files_to_cleanup(&self, video_id: VideoId) -> Result<Vec<PathBuf>> {
        let con = self.con.lock().unwrap();
        let mut stmt =
            con.prepare("SELECT root_name, path FROM videos_cleanup WHERE video_id = ?1")?;

        let result = stmt
            .query_map([video_id.id()], |row| self.to_cleanup_path(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    fn to_video(&self, row: &Row<'_>) -> rusqlite::Result<Video> {
        let video_id = row.get("video_id").map(VideoId::new)?;
        let root_id: i64 = row.get("root_id").unwrap_or(1);
        let root = self.get_root_by_id(root_id);

        let relative_path: String = row.get("video_path_b64")?;
        let relative_path = path_encoding::from_base64(&relative_path)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        let sandbox_path = root.sandbox_path.join(&relative_path);
        let host_path = root.host_path.join(&relative_path);

        let ordering_ts = row.get("ordering_ts").expect("must have ordering_ts");

        let stream_duration = row
            .get("stream_duration")
            .ok()
            .and_then(|x: i64| TimeDelta::try_milliseconds(x));

        let video_codec = row.get("video_codec").ok();

        let transcoded_path = row
            .get("transcoded_path")
            .map(|p: String| self.cache_dir_base_path.join(p))
            .ok();

        std::result::Result::Ok(Video {
            video_id,
            path: FlatpakPathBuf::build(host_path, sandbox_path),
            ordering_ts,
            stream_duration,
            video_codec,
            transcoded_path,
        })
    }

    fn to_cleanup_path(&self, row: &Row<'_>) -> rusqlite::Result<PathBuf> {
        let root_name: String = row.get("root_name")?;

        row.get("path")
            .and_then(|p: String| match root_name.as_str() {
                // FIXME don't forget the thumbnail path
                "cache" => std::result::Result::Ok(self.cache_dir_base_path.join(p)),
                "data" => std::result::Result::Ok(self.data_dir_base_path.join(p)),
                _ => Err(rusqlite::Error::InvalidPath(p.into())),
            })
    }

    pub fn remove(&mut self, video_id: VideoId) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare("DELETE FROM videos WHERE video_id = ?1")?;

        stmt.execute([video_id.id()])?;

        Ok(())
    }
}
