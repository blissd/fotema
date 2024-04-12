// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::PictureId;
use crate::video::VideoId;

use crate::Error::*;
use crate::Result;

use chrono::*;
use rusqlite::Connection;
use std::fmt::Display;
use std::path;
use std::path::PathBuf;

/// Database ID of a visual item
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualId(i64);

impl VisualId {
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for VisualId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A visual artefact, such as a photo or a video (or in some cases both at once).
#[derive(Debug, Clone)]
pub struct Visual {
    /// Full path from library root.
    pub visual_id: VisualId,

    /// Path to thumbnail. If both a picture and a video are present, then this will
    /// be the picture thumbnail path.
    pub thumbnail_path: PathBuf,

    pub video_id: Option<VideoId>,

    pub video_path: Option<PathBuf>,

    pub picture_id: Option<PictureId>,

    pub picture_path: Option<PathBuf>,

    /// Temporal ordering
    pub order_by_ts: DateTime<Utc>,

    is_selfie: Option<bool>,
}

impl Visual {
    pub fn is_selfie(&self) -> bool {
        self.is_selfie.is_some_and(|x| x)
    }

    pub fn is_motion_photo(&self) -> bool {
        self.picture_id.is_some() && self.video_id.is_some()
    }

    pub fn is_photo_only(&self) -> bool {
        self.picture_id.is_some() && self.video_id.is_none()
    }

    pub fn is_video_only(&self) -> bool {
        self.picture_id.is_none() && self.video_id.is_some()
    }
}

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug)]
pub struct Repository {
    /// Base path to picture library on file system
    library_base_path: path::PathBuf,

    video_thumbnail_base_path: path::PathBuf,

    photo_thumbnail_base_path: path::PathBuf,

    /// Connection to backing Sqlite database.
    con: rusqlite::Connection,
}

impl Repository {
    pub fn open_in_memory(
        library_base_path: &path::Path,
        photo_thumbnail_base_path: &path::Path,
        video_thumbnail_base_path: &path::Path,
    ) -> Result<Repository> {
        let con = Connection::open_in_memory().map_err(|e| RepositoryError(e.to_string()))?;

        let repo = Repository {
            library_base_path: path::PathBuf::from(library_base_path),
            video_thumbnail_base_path: path::PathBuf::from(video_thumbnail_base_path),
            photo_thumbnail_base_path: path::PathBuf::from(photo_thumbnail_base_path),
            con,
        };
        Ok(repo)
    }

    /// Builds a Repository and creates operational tables.
    pub fn open(
        library_base_path: &path::Path,
        photo_thumbnail_base_path: &path::Path,
        video_thumbnail_base_path: &path::Path,
        db_path: &path::Path,
    ) -> Result<Repository> {
        let con = Connection::open(db_path).map_err(|e| RepositoryError(e.to_string()))?;
        let repo = Repository {
            library_base_path: path::PathBuf::from(library_base_path),
            video_thumbnail_base_path: path::PathBuf::from(video_thumbnail_base_path),
            photo_thumbnail_base_path: path::PathBuf::from(photo_thumbnail_base_path),
            con,
        };
        Ok(repo)
    }

    /// Gets all visual artefacts.
    pub fn all(&self) -> Result<Vec<Visual>> {
        let mut stmt = self
            .con
            .prepare(
                "SELECT
                    visual_id,

                    picture_id,
                    pictures.picture_path AS picture_path,
                    pictures.preview_path AS picture_thumbnail,
                    pictures.order_by_ts AS picture_order_by_ts,

                    video_id,
                    videos.video_path AS video_path,
                    videos.preview_path AS video_thumbnail,
                    videos.created_ts AS video_created_ts
                FROM visual
                WHERE COALESCE(pictures.preview_path, videos.preview_path) IS NOT NULL,
                LEFT JOIN pictures USING(picture_id),
                LEFT JOIN videos USING(video_id)",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let visual_id = row
                    .get(0)
                    .map(|x| VisualId(x))
                    .expect("Must have visual_id");

                let picture_id: Option<PictureId> = row.get(1).map(|x| PictureId::new(x)).ok();
                let picture_path: Option<PathBuf> =
                    row.get(2).map(|x: String| PathBuf::from(x)).ok();
                let picture_path = picture_path.map(|x| self.library_base_path.join(x));
                let picture_thumbnail: Option<PathBuf> =
                    row.get(3).map(|x: String| PathBuf::from(x)).ok();
                let picture_order_by_ts: Option<DateTime<Utc>> = row.get(4).ok();

                let video_id: Option<VideoId> = row.get(5).map(|x| VideoId::new(x)).ok();
                let video_path: Option<PathBuf> = row.get(6).map(|x: String| PathBuf::from(x)).ok();
                let video_path = video_path.map(|x| self.library_base_path.join(x));
                let video_thumbnail: Option<PathBuf> =
                    row.get(7).map(|x: String| PathBuf::from(x)).ok();
                let video_created_ts: Option<DateTime<Utc>> = row.get(8).ok();

                let thumbnail_path = picture_thumbnail
                    .map(|x| self.photo_thumbnail_base_path.join(x))
                    .or_else(|| video_thumbnail)
                    .map(|x| self.video_thumbnail_base_path.join(x))
                    .expect("Must have a thumbnail");

                let order_by_ts = picture_order_by_ts
                    .or_else(|| video_created_ts)
                    .expect("Must have order_by_ts");

                let v = Visual {
                    visual_id,
                    thumbnail_path,
                    picture_id,
                    picture_path,
                    video_id,
                    video_path,
                    order_by_ts,
                    is_selfie: None, // TODO get real value
                };
                Ok(v)
            })
            .map_err(|e| RepositoryError(e.to_string()))?;

        // Would like to return an iterator... but Rust is defeating me.
        let mut visuals = Vec::new();
        for vis in iter.flatten() {
            visuals.push(vis);
        }

        Ok(visuals)
    }
}
