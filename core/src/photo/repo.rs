// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::FlatpakPathBuf;
use crate::ScannedFile;
use crate::path_encoding;
use crate::people::model::{DetectedFace, FaceDetectionCandidate, FaceId, Rect};
use crate::photo::model::{Picture, PictureId};

use super::Metadata;
use super::metadata;
use super::model::MotionPhotoVideo;
use super::motion_photo;
use anyhow::{Result, bail};
use rusqlite;
use rusqlite::Row;
use rusqlite::params;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tracing::error;

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Base path to library
    library_base_dir: FlatpakPathBuf,

    /// Base path cache directory for motion photo videos
    cache_dir_base_path: PathBuf,

    /// Base path cache directory for motion photo videos
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
        if !library_base_dir.sandbox_path.is_dir() {
            bail!("{:?} is not a directory", library_base_dir);
        }

        let repo = Repository {
            library_base_dir: library_base_dir.clone(),
            cache_dir_base_path: cache_dir_base_path.into(),
            data_dir_base_path: data_dir_base_path.into(),
            con,
        };

        Ok(repo)
    }

    pub fn add_metadatas(&mut self, pics: Vec<(PictureId, Metadata)>) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut update_pictures = tx.prepare_cached(
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

            let mut update_geo = tx.prepare_cached(
                "INSERT INTO pictures_geo (
                    picture_id,
                    latitude,
                    longitude
                ) VALUES (
                    ?1, ?2, ?3
                ) ON CONFLICT (picture_id) DO UPDATE SET
                    latitude = ?2,
                    longitude = ?3
                ",
            )?;

            for (picture_id, metadata) in pics {
                update_pictures.execute(params![
                    picture_id.id(),
                    metadata::VERSION,
                    metadata.created_at,
                    metadata.modified_at,
                    metadata.is_selfie(),
                    metadata.content_id,
                    metadata.orientation.map(|x| x as u8),
                ])?;

                if let Some(location) = metadata.location {
                    // Belts and braces.
                    // SQLite will treat a "nan" (not-a-number) as a null and cause
                    // the not-null constraint to be violated.
                    let latitude = location.latitude.to_f64_safe();
                    let longitude = location.longitude.to_f64_safe();
                    if latitude.is_some() && longitude.is_some() {
                        update_geo.execute(params![picture_id.id(), latitude, longitude,])?;
                    }
                }
            }
        }

        tx.commit()?;
        Ok(())
    }

    pub fn mark_broken(&mut self, picture_id: &PictureId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
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
                    fs_modified_ts,
                    picture_path_b64,
                    picture_path_lossy,
                    link_path_b64,
                    link_path_lossy
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6
                ) ON CONFLICT (picture_path_b64) DO UPDATE SET
                    fs_created_ts = ?1,
                    fs_modified_ts = ?2
                ",
            )?;

            for scanned_file in pics {
                if let ScannedFile::Photo(info) = scanned_file {
                    // convert to relative path before saving to database
                    let picture_path = info
                        .path
                        .strip_prefix(&self.library_base_dir.sandbox_path)?;
                    let picture_path_b64 = path_encoding::to_base64(picture_path);

                    // Path without suffix so sibling pictures and videos can be related
                    let link_path = picture_path
                        .file_stem()
                        .and_then(|x| x.to_str())
                        .expect("Must exist");

                    let link_path = picture_path.with_file_name(link_path);
                    let link_path_b64 = path_encoding::to_base64(&link_path);

                    pic_insert_stmt.execute(params![
                        info.fs_created_at,
                        info.fs_modified_at,
                        picture_path_b64,
                        picture_path.to_string_lossy(),
                        link_path_b64,
                        link_path.to_string_lossy(),
                    ])?;
                } else {
                    error!("Expected a photo, but got: {:?}", scanned_file);
                }
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
                    COALESCE(
                        pictures.exif_created_ts,
                        pictures.exif_modified_ts,
                        pictures.fs_created_ts,
                        pictures.fs_modified_ts,
                        CURRENT_TIMESTAMP
                      ) AS ordering_ts,
                    pictures.is_selfie
                FROM pictures
                WHERE COALESCE(is_broken, FALSE) IS FALSE
                ORDER BY ordering_ts ASC",
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
                    COALESCE(
                        pictures.exif_created_ts,
                        pictures.exif_modified_ts,
                        pictures.fs_created_ts,
                        pictures.fs_modified_ts,
                        CURRENT_TIMESTAMP
                      ) AS ordering_ts,
                    pictures.is_selfie
                FROM pictures
                WHERE metadata_version < ?1
                AND COALESCE(is_broken, FALSE) IS FALSE
                ORDER BY ordering_ts ASC",
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
                    COALESCE(
                        pictures.exif_created_ts,
                        pictures.exif_modified_ts,
                        pictures.fs_created_ts,
                        pictures.fs_modified_ts,
                        CURRENT_TIMESTAMP
                      ) AS ordering_ts,
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

    /// Gets paths of files to delete when a picture is no longer present.
    pub fn find_files_to_cleanup(&self, picture_id: PictureId) -> Result<Vec<PathBuf>> {
        let con = self.con.lock().unwrap();
        let mut stmt =
            con.prepare("SELECT root_name, path FROM pictures_cleanup WHERE picture_id = ?1")?;

        let result = stmt
            .query_map([picture_id.id()], |row| self.to_cleanup_path(row))?
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
                        rotation,
                        transcoded_path
                    ) VALUES (
                        ?1, ?2, ?3, ?4, ?5, ?6, ?7
                    ) ON CONFLICT (picture_id) DO UPDATE SET
                        extract_version = ?2,
                        video_path = ?3,
                        duration_millis = ?4,
                        video_codec = ?5,
                        rotation = ?6,
                        transcoded_path = ?7
                    ",
                )?;

                // convert to relative path before saving to database
                // path relative to cache directory so no need to base64 encode
                let video_path = video.path.strip_prefix(&self.cache_dir_base_path).ok();
                let transcoded_path = video
                    .transcoded_path
                    .as_ref()
                    .and_then(|x| x.strip_prefix(&self.cache_dir_base_path).ok());

                stmt.execute(params![
                    picture_id.id(),
                    motion_photo::VERSION,
                    video_path.as_ref().map(|p| p.to_str()),
                    video.duration.map(|x| x.num_milliseconds()),
                    video.video_codec,
                    video.rotation,
                    transcoded_path.as_ref().map(|p| p.to_string_lossy()),
                ])?;
            } else {
                let mut stmt = tx.prepare(
                    "INSERT INTO motion_photos (
                    picture_id,
                    extract_version,
                    video_path,
                    duration_millis,
                    video_codec,
                    transcoded_path
                ) VALUES (
                    ?1, ?2, NULL, NULL, NULL, NULL
                ) ON CONFLICT (picture_id) DO UPDATE SET
                    extract_version = ?2,
                    video_path = NULL,
                    duration_millis = NULL,
                    video_codec = NULL,
                    transcoded_path = NULL
                ",
                )?;

                stmt.execute(params![picture_id.id(), motion_photo::VERSION,])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    fn to_picture(&self, row: &Row<'_>) -> rusqlite::Result<Picture> {
        let picture_id = row.get("picture_id").map(PictureId::new)?;

        let relative_path: String = row.get("picture_path_b64")?;
        let relative_path = path_encoding::from_base64(&relative_path)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        let sandbox_path = self.library_base_dir.sandbox_path.join(&relative_path);
        let host_path = self.library_base_dir.host_path.join(&relative_path);

        let ordering_ts = row.get("ordering_ts").expect("must have ordering_ts");
        let is_selfie = row.get("is_selfie").ok();

        std::result::Result::Ok(Picture {
            picture_id,
            path: FlatpakPathBuf::build(host_path, sandbox_path),
            ordering_ts,
            is_selfie,
        })
    }

    fn to_cleanup_path(&self, row: &Row<'_>) -> rusqlite::Result<PathBuf> {
        let root_name: String = row.get("root_name")?;

        row.get("path")
            .and_then(|p: String| match root_name.as_str() {
                // FIXME what about thumbnail path?
                "cache" => std::result::Result::Ok(self.cache_dir_base_path.join(p)),
                "data" => std::result::Result::Ok(self.data_dir_base_path.join(p)),
                _ => Err(rusqlite::Error::InvalidPath(p.into())),
            })
    }

    pub fn remove(&mut self, picture_id: PictureId) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare("DELETE FROM pictures WHERE picture_id = ?1")?;

        stmt.execute([picture_id.id()])?;

        Ok(())
    }

    /// Find all people and associated face.
    /// FIXME move to people repo
    pub fn find_people_for_thumbnails(&self) -> Result<Vec<(FlatpakPathBuf, DetectedFace)>> {
        let con = self.con.lock().unwrap();

        // NOTE: this is non-standard SQL that might not work in DBs that aren't SQLite.
        let mut stmt = con.prepare(
            "SELECT
                face_id,
                detected_at,

                is_source_original,
                pictures.picture_path_b64 AS picture_path_b64,

                bounds_path,

                bounds_x,
                bounds_y,
                bounds_width,
                bounds_height,

                right_eye_x,
                right_eye_y,

                left_eye_x,
                left_eye_y,

                nose_x,
                nose_y,

                right_mouth_corner_x,
                right_mouth_corner_y,

                left_mouth_corner_x,
                left_mouth_corner_y,

                confidence
            FROM  pictures_faces AS faces
            INNER JOIN pictures USING (picture_id)
            WHERE faces.person_id IS NOT NULL
            AND faces.is_thumbnail IS TRUE",
        )?;

        let result: Vec<(FlatpakPathBuf, DetectedFace)> = stmt
            .query_map([], |row| {
                Ok((self.to_library_path(row)?, self.to_detected_face(row)?))
            })?
            .flatten()
            .collect();

        Ok(result)
    }

    fn to_library_path(&self, row: &Row<'_>) -> rusqlite::Result<FlatpakPathBuf> {
        let relative_path: String = row.get("picture_path_b64")?;
        let relative_path = path_encoding::from_base64(&relative_path)
            .map_err(|_| rusqlite::Error::InvalidQuery)?;

        let sandbox_path = self.library_base_dir.sandbox_path.join(&relative_path);
        let host_path = self.library_base_dir.host_path.join(&relative_path);

        std::result::Result::Ok(FlatpakPathBuf {
            host_path,
            sandbox_path,
        })
    }

    /// FIXME a copy-n-paste from people repo :-()
    fn to_detected_face(&self, row: &Row<'_>) -> rusqlite::Result<DetectedFace> {
        let face_id = row.get("face_id").map(FaceId::new)?;

        let face_path = row
            .get("bounds_path")
            .map(|p: String| self.data_dir_base_path.join(p))?;

        let bounds = Rect {
            x: row.get("bounds_x")?,
            y: row.get("bounds_y")?,
            width: row.get("bounds_width")?,
            height: row.get("bounds_height")?,
        };

        let right_eye_x = row.get("right_eye_x")?;
        let right_eye_y = row.get("right_eye_y")?;

        let left_eye_x = row.get("left_eye_x")?;
        let left_eye_y = row.get("left_eye_y")?;

        let nose_x = row.get("nose_x")?;
        let nose_y = row.get("nose_y")?;

        let right_mouth_corner_x = row.get("right_mouth_corner_x")?;
        let right_mouth_corner_y = row.get("right_mouth_corner_y")?;

        let left_mouth_corner_x = row.get("left_mouth_corner_x")?;
        let left_mouth_corner_y = row.get("left_mouth_corner_y")?;

        let confidence = row.get("confidence")?;

        let detected_at = row.get("detected_at")?;

        let is_source_original: bool = row.get("is_source_original")?;

        let face = DetectedFace {
            face_id,
            face_path,
            is_source_original,
            bounds,
            right_eye: (right_eye_x, right_eye_y),
            left_eye: (left_eye_x, left_eye_y),
            nose: (nose_x, nose_y),
            right_mouth_corner: (right_mouth_corner_x, right_mouth_corner_y),
            left_mouth_corner: (left_mouth_corner_x, left_mouth_corner_y),
            confidence,
            detected_at,
        };

        std::result::Result::Ok(face)
    }

    /// Gets all pictures that haven't been scanned for faces.
    /// This method is not on the people repo because I don't what that repo
    /// to need a pic_base_dir.
    /// FIXME move to people repo
    pub fn find_face_detection_candidates(&self) -> Result<Vec<FaceDetectionCandidate>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                    pictures.picture_id,
                    pictures.picture_path_b64,
                    COALESCE(
                        pictures.exif_created_ts,
                        pictures.exif_modified_ts,
                        pictures.fs_created_ts,
                        pictures.fs_modified_ts,
                        CURRENT_TIMESTAMP
                    ) AS ordering_ts
                FROM pictures
                LEFT OUTER JOIN pictures_face_scans USING (picture_id)
                WHERE pictures_face_scans.picture_id IS NULL
                AND COALESCE(pictures.is_broken, FALSE) IS FALSE
                ORDER BY ordering_ts DESC",
        )?;

        let result = stmt
            .query_map([], |row| self.to_face_detection_candidate(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    /// Gets all pictures that haven't been scanned for faces.
    /// This method is not on the people repo because I don't what that repo
    /// to need a pic_base_dir.
    /// FIXME move to people repo
    pub fn get_face_detection_candidate(
        &self,
        picture_id: &PictureId,
    ) -> Result<Option<FaceDetectionCandidate>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                    pictures.picture_id,
                    pictures.picture_path_b64
                FROM pictures
                WHERE pictures.picture_id = ?1",
        )?;

        let result = stmt
            .query_map([picture_id.id()], |row| {
                self.to_face_detection_candidate(row)
            })?
            .flatten()
            .nth(0);

        Ok(result)
    }

    /// FIXME move to people repo
    fn to_face_detection_candidate(
        &self,
        row: &Row<'_>,
    ) -> rusqlite::Result<FaceDetectionCandidate> {
        let picture_id = row.get("picture_id").map(PictureId::new)?;

        let relative_path = row
            .get("picture_path_b64")
            .ok()
            .and_then(|x: String| path_encoding::from_base64(&x).ok());

        let host_path = relative_path
            .as_ref()
            .map(|x| self.library_base_dir.host_path.join(x));

        let sandbox_path = relative_path.map(|x| self.library_base_dir.sandbox_path.join(x));

        Ok(FaceDetectionCandidate {
            picture_id,
            host_path: host_path.expect("Must have host path"),
            sandbox_path: sandbox_path.expect("Must have sandbox path"),
        })
    }
}
