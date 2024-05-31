// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::{Picture, PictureId, ScannedFile};

use super::metadata;
use super::model::MotionPhotoVideo;
use super::motion_photo;
use super::Metadata;
use crate::path_encoding;
use anyhow::*;
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

    /// Base path for photo thumbnails and motion photo videos
    cache_dir_base_path: PathBuf,

    /// Connection to backing Sqlite database.
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn open(
        library_base_path: &Path,
        cache_dir_base_path: &Path,
        con: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Repository> {
        if !library_base_path.is_dir() {
            bail!("{:?} is not a directory", library_base_path);
        }

        let library_base_path = PathBuf::from(library_base_path);
        let cache_dir_base_path = PathBuf::from(cache_dir_base_path);

        let repo = Repository {
            library_base_path,
            cache_dir_base_path,
            con,
        };

        Ok(repo)
    }

    pub fn add_metadatas(&mut self, pics: Vec<(PictureId, Metadata)>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare(
                "UPDATE pictures
                SET
                    metadata_version = ?2,
                    exif_created_ts = ?3,
                    exif_modified_ts = ?4,
                    is_selfie = ?5,
                    content_id = ?6,
                    orientation = ?7
                WHERE picture_id = ?1",
            )?;

            for (picture_id, metadata) in pics {
                stmt.execute(params![
                    picture_id.id(),
                    metadata::VERSION,
                    metadata.created_at,
                    metadata.modified_at,
                    metadata.is_selfie(),
                    metadata.content_id,
                    metadata.orientation.map(|x| x as u8),
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn add_thumbnail(&mut self, picture_id: &PictureId, thumbnail_path: &Path) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare(
                "UPDATE pictures
                SET
                    thumbnail_path = ?2,
                    is_broken = FALSE
                WHERE picture_id = ?1",
            )?;

            // convert to relative path before saving to database
            let thumbnail_path = thumbnail_path.strip_prefix(&self.cache_dir_base_path).ok();

            stmt.execute(params![
                picture_id.id(),
                thumbnail_path.as_ref().map(|p| p.to_str()),
            ])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn mark_broken(&mut self, picture_id: &PictureId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare(
                "UPDATE pictures
                SET
                    is_broken = TRUE
                WHERE picture_id = ?1",
            )?;

            stmt.execute(params![picture_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Add all Pictures received from a vector.
    pub fn add_all(&mut self, pics: &Vec<ScannedFile>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut pic_insert_stmt = tx.prepare_cached(
                "INSERT INTO pictures (
                    fs_created_ts,
                    picture_path_b64,
                    picture_path_lossy,
                    link_path_b64,
                    link_path_lossy
                ) VALUES (
                    ?1, ?2, $3, $4, $5
                ) ON CONFLICT (picture_path_b64) DO UPDATE SET
                    fs_created_ts=?1
                ",
            )?;

            for pic in pics {
                // convert to relative path before saving to database
                let picture_path = pic.path.strip_prefix(&self.library_base_path)?;
                let picture_path_b64 = path_encoding::to_base64(&picture_path);

                // Path without suffix so sibling pictures and videos can be related
                let link_path = picture_path
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .expect("Must exist");

                let link_path = picture_path.with_file_name(link_path);
                let link_path_b64 = path_encoding::to_base64(&link_path);

                pic_insert_stmt.execute(params![
                    pic.fs_created_at,
                    picture_path_b64,
                    picture_path.to_string_lossy(),
                    link_path_b64,
                    link_path.to_string_lossy(),
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Gets all pictures in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Picture>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                    pictures.picture_id,
                    pictures.picture_path_b64,
                    pictures.thumbnail_path,
                    pictures.fs_created_ts,
                    pictures.exif_created_ts,
                    pictures.exif_modified_ts,
                    pictures.is_selfie
                FROM pictures
                WHERE COALESCE(is_broken, FALSE) IS FALSE
                ORDER BY COALESCE(exif_created_ts, fs_created_ts) ASC",
        )?;

        let result = stmt
            .query_map([], |row| self.to_picture(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    /// Gets all pictures that haven't had their metadata extracted.
    /// Will return all pictures that are not broken and have a metadata version
    /// lower than the current metadata scanner.
    pub fn find_need_metadata_update(&self) -> Result<Vec<Picture>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                    pictures.picture_id,
                    pictures.picture_path_b64,
                    pictures.thumbnail_path,
                    pictures.fs_created_ts,
                    pictures.exif_created_ts,
                    pictures.exif_modified_ts,
                    pictures.is_selfie
                FROM pictures
                WHERE metadata_version < ?1
                AND COALESCE(is_broken, FALSE) IS FALSE
                ORDER BY COALESCE(exif_created_ts, fs_created_ts) ASC",
        )?;

        let result = stmt
            .query_map([metadata::VERSION], |row| self.to_picture(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    /// Gets all pictures that haven't been inspected for containing a motion photo.
    pub fn find_need_motion_photo_extract(&self) -> Result<Vec<Picture>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                    pictures.picture_id,
                    pictures.picture_path_b64,
                    pictures.thumbnail_path,
                    pictures.fs_created_ts,
                    pictures.exif_created_ts,
                    pictures.exif_modified_ts,
                    pictures.is_selfie
                FROM pictures
                FULL OUTER JOIN motion_photos USING (picture_id)
                WHERE COALESCE(motion_photos.extract_version, 0) < ?1
                AND COALESCE(is_broken, FALSE) IS FALSE",
        )?;

        let result = stmt
            .query_map([motion_photo::VERSION], |row| self.to_picture(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    pub fn add_motion_photo_video(
        &mut self,
        picture_id: &PictureId,
        video: Option<MotionPhotoVideo>,
    ) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            if let Some(video) = video {
                let mut stmt = tx.prepare(
                    "INSERT INTO motion_photos (
                        picture_id,
                        extract_version,
                        video_path,
                        duration_millis,
                        video_codec,
                        rotation
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6
                    ) ON CONFLICT (picture_id) DO UPDATE SET
                        extract_version = ?2,
                        video_path = ?3,
                        duration_millis = ?4,
                        video_codec = ?5,
                        rotation = ?6
                    ",
                )?;

                // convert to relative path before saving to database
                // path relative to cache directory so no need to base64 encode
                let video_path = video.path.strip_prefix(&self.cache_dir_base_path).ok();

                stmt.execute(params![
                    picture_id.id(),
                    motion_photo::VERSION,
                    video_path.as_ref().map(|p| p.to_str()),
                    video.duration.map(|x| x.num_milliseconds()),
                    video.video_codec,
                    video.rotation,
                ])?;
            } else {
                let mut stmt = tx.prepare(
                    "INSERT INTO motion_photos (
                    picture_id,
                    extract_version,
                    video_path,
                    duration_millis,
                    video_codec
                ) VALUES (
                    ?1, ?2, NULL, NULL, NULL
                ) ON CONFLICT (picture_id) DO UPDATE SET
                    extract_version = ?2,
                    video_path = NULL,
                    duration_millis = NULL,
                    video_codec = NULL
                ",
                )?;

                stmt.execute(params![picture_id.id(), motion_photo::VERSION,])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    fn to_picture(&self, row: &Row<'_>) -> rusqlite::Result<Picture> {
        let picture_id = row.get("picture_id").map(|x| PictureId::new(x))?;

        let picture_path: String = row.get("picture_path_b64")?;
        let picture_path =
            path_encoding::from_base64(&picture_path).map_err(|_| rusqlite::Error::InvalidQuery)?;
        let picture_path = self.library_base_path.join(picture_path);

        let thumbnail_path = row
            .get("thumbnail_path")
            .map(|p: String| self.cache_dir_base_path.join(p))
            .ok();

        let fs_created_at = row.get("fs_created_ts")?;
        let exif_created_at = row.get("exif_created_ts").ok();
        let exif_modified_at = row.get("exif_modified_ts").ok();
        let is_selfie = row.get("is_selfie").ok();

        std::result::Result::Ok(Picture {
            picture_id,
            path: picture_path,
            thumbnail_path,
            fs_created_at,
            exif_created_at,
            exif_modified_at,
            is_selfie,
        })
    }

    pub fn remove(&mut self, picture_id: PictureId) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare("DELETE FROM pictures WHERE picture_id = ?1")?;

        stmt.execute([picture_id.id()])?;

        Ok(())
    }
}
