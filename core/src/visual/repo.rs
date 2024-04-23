// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::PictureId;
use crate::video::VideoId;
use crate::visual::model::{Visual, VisualId};

use crate::Error::*;
use crate::Result;

use chrono::*;
use rusqlite;
use std::path;
use std::path::PathBuf;
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
                    .map(|x| VisualId::new(x))
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
            .query_map([visual_id.id()], |row| {
                let visual_id = row
                    .get(0)
                    .map(|x| VisualId::new(x))
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
