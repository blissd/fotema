// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::PictureId;
use crate::video::VideoId;
use crate::visual::model::{Visual, VisualId};

use anyhow::*;
use chrono::*;
use rusqlite;
use rusqlite::Row;
use std::path;
use std::path::PathBuf;
use std::result::Result::Ok;
use std::sync::{Arc, Mutex};

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Base path to picture library on file system
    library_base_path: path::PathBuf,

    /// Base path for thumbnails and transcoded videos
    thumbnail_base_path: path::PathBuf,

    /// Connection to backing Sqlite database.
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn open(
        library_base_path: &path::Path,
        thumbnail_base_path: &path::Path,
        con: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Repository> {
        let repo = Repository {
            library_base_path: path::PathBuf::from(library_base_path),
            thumbnail_base_path: path::PathBuf::from(thumbnail_base_path),
            con,
        };
        Ok(repo)
    }

    /// Gets all visual artefacts.
    pub fn all(&self) -> Result<Vec<Visual>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                    visual_id,
                    link_path AS stem_path,

                    picture_id,
                    picture_path,
                    picture_thumbnail,
                    is_selfie,

                    video_id,
                    video_path,
                    video_thumbnail,

                    created_ts,
                    is_ios_live_photo,

                    video_transcoded_path,
                    is_transcode_required,
                    duration_millis
                FROM visual
                ORDER BY created_ts ASC",
        )?;

        let result = stmt.query_map([], |row| self.to_visual(row))?;
        let visuals = result.flatten().collect();
        Ok(visuals)
    }

    pub fn get(&mut self, visual_id: VisualId) -> Result<Option<Visual>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                    visual_id,
                    link_path AS stem_path,

                    picture_id,
                    picture_path,
                    picture_thumbnail,
                    is_selfie,

                    video_id,
                    video_path,
                    video_thumbnail,

                    created_ts,
                    is_ios_live_photo

                    video_transcoded_path,
                    is_transcode_required,
                    video_duration
                FROM visual
                AND visual.visual_id = ?1",
        )?;

        let iter = stmt.query_map([visual_id.id()], |row| self.to_visual(row))?;
        let head = iter.flatten().nth(0);
        Ok(head)
    }

    fn to_visual(&self, row: &Row<'_>) -> rusqlite::Result<Visual> {
        let visual_id = row
            .get("visual_id")
            .map(|x| VisualId::new(x))
            .expect("Must have visual_id");

        let stem_path: PathBuf = row
            .get("stem_path")
            .map(|x: String| PathBuf::from(x))
            .expect("Stem path");

        let picture_id: Option<PictureId> = row.get("picture_id").map(|x| PictureId::new(x)).ok();

        let picture_path: Option<PathBuf> = row
            .get("picture_path")
            .map(|x: String| PathBuf::from(x))
            .ok();

        let picture_path = picture_path.map(|x| self.library_base_path.join(x));

        let picture_thumbnail: Option<PathBuf> = row
            .get("picture_thumbnail")
            .map(|x: String| PathBuf::from(x))
            .map(|x| self.thumbnail_base_path.join(x))
            .ok();

        let is_selfie: Option<bool> = row.get("is_selfie").ok();

        let video_id: Option<VideoId> = row.get("video_id").map(|x| VideoId::new(x)).ok();

        let video_path: Option<PathBuf> =
            row.get("video_path").map(|x: String| PathBuf::from(x)).ok();

        let video_path = video_path.map(|x| self.library_base_path.join(x));

        let video_thumbnail: Option<PathBuf> = row
            .get("video_thumbnail")
            .map(|x: String| PathBuf::from(x))
            .map(|x| self.thumbnail_base_path.join(x))
            .ok();

        let thumbnail_path: Option<PathBuf> = picture_thumbnail.or(video_thumbnail);

        let created_at: DateTime<Utc> = row.get("created_ts").ok().expect("Must have created_ts");

        let is_ios_live_photo: Option<bool> = row.get("is_ios_live_photo").ok();

        let is_ios_live_photo = is_ios_live_photo.is_some_and(|x| x);

        let video_transcoded_path: Option<PathBuf> = row
            .get("video_transcoded_path")
            .ok()
            .map(|x: String| PathBuf::from(x))
            .map(|x| self.thumbnail_base_path.join(x));

        let is_transcode_required: Option<bool> = row.get("is_transcode_required").ok();

        let video_duration: Option<TimeDelta> = row
            .get("duration_millis")
            .ok()
            .and_then(|x| TimeDelta::try_milliseconds(x));

        let v = Visual {
            visual_id,
            parent_path: stem_path
                .parent()
                .map(|x| PathBuf::from(x))
                .expect("Parent path"),
            thumbnail_path,
            picture_id,
            picture_path,
            video_id,
            video_path,
            created_at,
            is_selfie,
            is_ios_live_photo,
            video_transcoded_path,
            is_transcode_required,
            video_duration,
        };
        Ok(v)
    }
}
