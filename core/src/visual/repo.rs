// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::PictureId;
use crate::video::VideoId;

use crate::Error::*;
use crate::Result;
use crate::YearMonth;

use chrono::*;
use rusqlite;
use std::fmt::Display;
use std::path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Database ID of a visual item
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisualId(String);

impl VisualId {
    pub fn id(&self) -> &String {
        &self.0
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

    // Path to parent directory
    pub parent_path: PathBuf,

    /// Path to thumbnail. If both a picture and a video are present, then this will
    /// be the picture thumbnail path.
    pub thumbnail_path: PathBuf,

    pub video_id: Option<VideoId>,

    pub video_path: Option<PathBuf>,

    // Transcoded version of video_path of video_codec is not supported.
    pub video_transcoded_path: Option<PathBuf>,

    pub picture_id: Option<PictureId>,

    pub picture_path: Option<PathBuf>,

    /// EXIF or file system creation timestamp
    pub created_at: DateTime<Utc>,

    // Is this a selfie?
    is_selfie: Option<bool>,

    // Is this an iOS live photo?
    is_ios_live_photo: bool,

    // Does the video_code require the video is transcoded?
    pub is_transcode_required: Option<bool>,
}

impl Visual {
    pub fn path(&self) -> Option<&PathBuf> {
        self.picture_path
            .as_ref()
            .or_else(|| self.video_path.as_ref())
    }

    pub fn is_selfie(&self) -> bool {
        self.is_selfie.is_some_and(|x| x)
    }

    pub fn is_motion_photo(&self) -> bool {
        self.is_ios_live_photo
    }

    pub fn is_photo_only(&self) -> bool {
        self.picture_id.is_some() && self.video_id.is_none()
    }

    pub fn is_video_only(&self) -> bool {
        self.picture_id.is_none() && self.video_id.is_some()
    }

    pub fn year(&self) -> u32 {
        self.created_at.date_naive().year_ce().1
    }

    pub fn year_month(&self) -> YearMonth {
        let date = self.created_at.date_naive();
        let year = date.year();
        let month = date.month();
        let month = chrono::Month::try_from(u8::try_from(month).unwrap()).unwrap();
        YearMonth { year, month }
    }

    // TODO should really just compute this in photo_info.rs
    pub fn folder_name(&self) -> Option<String> {
        self.parent_path
            .file_name()
            .map(|x| x.to_string_lossy().to_string())
    }
}

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
        if !library_base_path.is_dir() {
            return Err(RepositoryError(format!(
                "{:?} is not a directory",
                library_base_path
            )));
        }

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
        let mut stmt = con
            .prepare(
                "SELECT
                    visual_id,
                    link_path AS stem_path,

                    picture_id,
                    picture_path,
                    picture_thumbnail,

                    video_id,
                    video_path,
                    video_thumbnail,

                    created_ts,
                    is_ios_live_photo,

                    video_transcoded_path,
                    is_transcode_required
                FROM visual
                ORDER BY created_ts ASC",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let visual_id = row
                    .get(0)
                    .map(|x| VisualId(x))
                    .expect("Must have visual_id");

                let stem_path: PathBuf = row
                    .get(1)
                    .map(|x: String| PathBuf::from(x))
                    .expect("Stem path");

                let picture_id: Option<PictureId> = row.get(2).map(|x| PictureId::new(x)).ok();

                let picture_path: Option<PathBuf> =
                    row.get(3).map(|x: String| PathBuf::from(x)).ok();

                let picture_path = picture_path.map(|x| self.library_base_path.join(x));

                let picture_thumbnail: Option<PathBuf> =
                    row.get(4).map(|x: String| PathBuf::from(x)).ok();

                let video_id: Option<VideoId> = row.get(5).map(|x| VideoId::new(x)).ok();

                let video_path: Option<PathBuf> = row.get(6).map(|x: String| PathBuf::from(x)).ok();

                let video_path = video_path.map(|x| self.library_base_path.join(x));

                let video_thumbnail: Option<PathBuf> =
                    row.get(7).map(|x: String| PathBuf::from(x)).ok();

                let thumbnail_path = picture_thumbnail
                    .map(|x| self.thumbnail_base_path.join(x))
                    .or_else(|| video_thumbnail)
                    .map(|x| self.thumbnail_base_path.join(x))
                    .expect("Must have a thumbnail");

                let created_at: DateTime<Utc> = row.get(8).expect("Must have created_ts");

                let is_ios_live_photo: bool = row.get(9).expect("must have is_ios_live_photo");

                let video_transcoded_path: Option<PathBuf> =
                    row.get(10).map(|x: String| PathBuf::from(x)).ok();

                let is_transcode_required: Option<bool> = row.get(11).ok();

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
                    is_selfie: None, // TODO get real value,
                    is_ios_live_photo,
                    video_transcoded_path,
                    is_transcode_required,
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

    pub fn get(&mut self, visual_id: VisualId) -> Result<Option<Visual>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con
            .prepare(
                "SELECT
                    visual_id,
                    link_path AS stem_path,

                    picture_id,
                    picture_path,
                    picture_thumbnail,

                    video_id,
                    video_path,
                    video_thumbnail,

                    created_ts,
                    is_ios_live_photo

                    video_transcoded_path,
                    is_transcode_required
                FROM visual
                AND visual.visual_id = ?1",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([visual_id.0], |row| {
                let visual_id = row
                    .get(0)
                    .map(|x| VisualId(x))
                    .expect("Must have visual_id");

                let stem_path: PathBuf = row
                    .get(1)
                    .map(|x: String| PathBuf::from(x))
                    .expect("Stem path");

                let picture_id: Option<PictureId> = row.get(2).map(|x| PictureId::new(x)).ok();

                let picture_path: Option<PathBuf> =
                    row.get(3).map(|x: String| PathBuf::from(x)).ok();

                let picture_path = picture_path.map(|x| self.library_base_path.join(x));

                let picture_thumbnail: Option<PathBuf> =
                    row.get(4).map(|x: String| PathBuf::from(x)).ok();

                let video_id: Option<VideoId> = row.get(5).map(|x| VideoId::new(x)).ok();

                let video_path: Option<PathBuf> = row.get(6).map(|x: String| PathBuf::from(x)).ok();

                let video_path = video_path.map(|x| self.library_base_path.join(x));

                let video_thumbnail: Option<PathBuf> =
                    row.get(7).map(|x: String| PathBuf::from(x)).ok();

                let thumbnail_path = picture_thumbnail
                    .map(|x| self.thumbnail_base_path.join(x))
                    .or_else(|| video_thumbnail)
                    .map(|x| self.thumbnail_base_path.join(x))
                    .expect("Must have a thumbnail");

                let created_at: DateTime<Utc> = row.get(8).ok().expect("Must have created_ts");

                let is_ios_live_photo: bool = row.get(9).expect("must have is_ios_live_photo");

                let video_transcoded_path: Option<PathBuf> =
                    row.get(10).map(|x: String| PathBuf::from(x)).ok();

                let is_transcode_required: Option<bool> = row.get(11).ok();

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
                    is_selfie: None, // TODO get real value
                    is_ios_live_photo,
                    video_transcoded_path,
                    is_transcode_required,
                };
                Ok(v)
            })
            .map_err(|e| RepositoryError(e.to_string()))?;

        let head = iter.flatten().nth(0);
        Ok(head)
    }
}
