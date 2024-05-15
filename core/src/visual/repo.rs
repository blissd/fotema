// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::PictureId;
use crate::video::VideoId;
use crate::visual::model::{PictureOrientation, Visual, VisualId};

use crate::path_encoding;
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
                    link_path_b64,

                    picture_id,
                    picture_path_b64,
                    picture_thumbnail,
                    picture_orientation,
                    is_selfie,

                    video_id,
                    video_path_b64,
                    video_thumbnail,

                    created_ts,
                    is_ios_live_photo,

                    video_transcoded_path,
                    is_transcode_required,
                    duration_millis,
                    video_rotation
                FROM visual
                ORDER BY created_ts ASC",
        )?;

        let result = stmt.query_map([], |row| self.to_visual(row))?;
        let visuals = result.flatten().collect();
        Ok(visuals)
    }

    fn to_visual(&self, row: &Row<'_>) -> rusqlite::Result<Visual> {
        let visual_id = row
            .get("visual_id")
            .map(|x| VisualId::new(x))
            .expect("Must have visual_id");

        let link_path: String = row.get("link_path_b64")?;
        let link_path =
            path_encoding::from_base64(&link_path).map_err(|_| rusqlite::Error::InvalidQuery)?;

        let picture_id: Option<PictureId> = row.get("picture_id").map(|x| PictureId::new(x)).ok();

        let picture_path: Option<PathBuf> = row
            .get("picture_path_b64")
            .ok()
            .and_then(|x: String| path_encoding::from_base64(&x).ok());

        let picture_path = picture_path.map(|x| self.library_base_path.join(x));

        let picture_thumbnail: Option<PathBuf> = row
            .get("picture_thumbnail")
            .map(|x: String| PathBuf::from(x))
            .map(|x| self.thumbnail_base_path.join(x))
            .ok();

        let picture_orientation: Option<PictureOrientation> = row
            .get("picture_orientation")
            .map(|x: u32| PictureOrientation::from(x))
            .ok();

        let is_selfie: Option<bool> = row.get("is_selfie").ok();

        let video_id: Option<VideoId> = row.get("video_id").map(|x| VideoId::new(x)).ok();

        let video_path: Option<PathBuf> = row
            .get("video_path_b64")
            .ok()
            .and_then(|x: String| path_encoding::from_base64(&x).ok());

        let video_path = video_path.map(|x| self.library_base_path.join(x));

        let video_thumbnail: Option<PathBuf> = row
            .get("video_thumbnail")
            .map(|x: String| PathBuf::from(x))
            .map(|x| self.thumbnail_base_path.join(x))
            .ok();

        let video_orientation: Option<PictureOrientation> = row
            .get("video_rotation")
            .map(|x: i32| PictureOrientation::from_degrees(x))
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
            parent_path: link_path
                .parent()
                .map(|x| PathBuf::from(x))
                .expect("Parent path"),
            thumbnail_path,
            picture_id,
            picture_path,
            picture_orientation,
            video_id,
            video_path,
            created_at,
            is_selfie,
            is_ios_live_photo,
            video_transcoded_path,
            video_orientation,
            is_transcode_required,
            video_duration,
        };
        Ok(v)
    }
}
