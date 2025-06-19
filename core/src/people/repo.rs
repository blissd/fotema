// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;

use crate::machine_learning::face_extractor;
use crate::path_encoding;
use crate::people::FaceId;
use crate::people::FaceToMigrate;
use crate::people::MigratedFace;
use crate::people::PersonId;
use crate::people::model;
use crate::people::model::PersonForRecognition;
use crate::people::model::Rect;

use anyhow::*;
use rusqlite;
use rusqlite::Row;
use rusqlite::params;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use std::sync::{Arc, Mutex};
use tracing::warn;

/// Repository of people data.
/// Repository is backed by a Sqlite database.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Cache direcctory
    cache_dir_base_path: PathBuf,

    /// Data directory
    data_dir_base_path: PathBuf,

    /// Connection to backing Sqlite database.
    con: Arc<Mutex<rusqlite::Connection>>,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn open(
        cache_dir_base_path: &Path,
        data_dir_base_path: &Path,
        con: Arc<Mutex<rusqlite::Connection>>,
    ) -> Result<Repository> {
        let cache_dir_base_path = PathBuf::from(cache_dir_base_path);
        let data_dir_base_path = PathBuf::from(data_dir_base_path);

        let repo = Repository {
            cache_dir_base_path,
            data_dir_base_path,
            con,
        };

        Ok(repo)
    }

    /// Deletes faces for a picture so a picture can be re-scanned and new faces.
    /// We must delete before re-scanning a picture for faces to avoid a unique constraint
    /// violation on the bounds_path.
    pub fn delete_faces(&self, picture_id: PictureId) -> Result<()> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "DELETE FROM pictures_faces
            WHERE pictures_faces.picture_id = ?1",
        )?;

        stmt.execute([picture_id.id()])?;

        Ok(())
    }

    /// Finds faces and people for the thumbnail bar.
    /// Faces are ordered from left to right, top to bottom.
    pub fn find_faces(
        &self,
        picture_id: &PictureId,
    ) -> Result<Vec<(model::Face, Option<model::Person>)>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                faces.face_id AS face_id,
                faces.thumbnail_path AS face_thumbnail_path,
                people.person_id AS person_id,
                people.name AS person_name,
                person_face.thumbnail_path AS person_thumbnail_path
            FROM pictures_faces AS faces
            LEFT OUTER JOIN people USING (person_id)
            LEFT OUTER JOIN pictures_faces AS person_face
                ON (person_face.person_id = faces.person_id AND person_face.is_thumbnail = TRUE)
            WHERE faces.picture_id = ?1 AND faces.is_ignored = FALSE
            ORDER BY faces.nose_x ASC, faces.nose_y ASC",
        )?;

        let result = stmt
            .query_map([picture_id.id()], |row| self.to_face_and_person(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    pub fn ignore_unknown_faces(&mut self, picture_id: PictureId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    is_ignored = TRUE
                WHERE picture_id = ?1 AND person_id IS NULL",
            )?;
            stmt.execute(params![picture_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn restore_ignored_faces(&mut self, picture_id: PictureId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    is_ignored = FALSE
                WHERE picture_id = ?1",
            )?;
            stmt.execute(params![picture_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_person(&self, person_id: PersonId) -> Result<Option<model::Person>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                p.person_id AS person_id,
                p.name AS person_name,
                f.thumbnail_path AS person_thumbnail_path
            FROM people AS p
            LEFT OUTER JOIN pictures_faces AS f
                ON (f.person_id = p.person_id AND f.is_thumbnail = TRUE)
            WHERE person_id = ?1",
        )?;

        let result: Option<model::Person> = stmt
            .query_map([person_id.id()], |row| self.to_person(row))?
            .flatten()
            .nth(0);

        Ok(result)
    }

    pub fn delete_person(&mut self, person_id: PersonId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    is_confirmed = FALSE,
                    is_thumbnail = FALSE
                WHERE person_id = ?1",
            )?;
            stmt.execute(params![person_id.id(),])?;

            let mut stmt = tx.prepare_cached("DELETE FROM people WHERE person_id = ?1")?;
            stmt.execute(params![person_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn rename_person(&mut self, person_id: PersonId, name: &str) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE people
                SET
                    name = ?2
                WHERE person_id = ?1",
            )?;
            stmt.execute(params![person_id.id(), name,])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn all_people(&self) -> Result<Vec<model::Person>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                p.person_id AS person_id,
                p.name AS person_name,
                f.thumbnail_path AS person_thumbnail_path
            FROM people AS p
            LEFT OUTER JOIN pictures_faces AS f
                ON (f.person_id = p.person_id AND f.is_thumbnail = TRUE)
            ORDER BY name ASC",
        )?;

        let result: Vec<model::Person> = stmt
            .query_map([], |row| self.to_person(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    /// All known people that must have a face recognition performed.
    /// Select the best face for recognition, where "best" is the face with
    /// the highest confidence for a face that the user has confirmed is a particular person.
    pub fn find_people_for_recognition(&self) -> Result<Vec<model::PersonForRecognition>> {
        let con = self.con.lock().unwrap();

        // NOTE: this is non-standard SQL that might not work in DBs that aren't SQLite.
        let mut stmt = con.prepare(
            "SELECT
                person_id,
                recognized_at,

                face_id,
                detected_at,

                is_source_original,

                bounds_path,
                thumbnail_path,

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

                max(confidence) AS confidence
            FROM  pictures_faces AS faces
            INNER JOIN people USING (person_id)
            WHERE faces.is_confirmed = TRUE
            GROUP BY faces.person_id",
        )?;

        let result: Vec<model::PersonForRecognition> = stmt
            .query_map([], |row| self.to_person_for_recognition(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    /// Find new faces as candidates for face recognition for a given person.
    /// Only returns faces that haven't been recognized before for the person.
    pub fn find_unknown_faces(&self) -> Result<Vec<model::DetectedFace>> {
        let con = self.con.lock().unwrap();

        // NOTE: this is non-standard SQL that might not work in DBs that aren't SQLite.
        let mut stmt = con.prepare(
            "SELECT
                face_id,
                detected_at,

                is_source_original,

                bounds_path,
                thumbnail_path,

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
            WHERE faces.person_id IS NULL
            AND faces.is_ignored = FALSE",
        )?;

        let result: Vec<model::DetectedFace> = stmt
            .query_map([], |row| self.to_detected_face(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    /// Finds all pictures that feature a known person.
    pub fn find_pictures_for_person(&self, person_id: PersonId) -> Result<Vec<PictureId>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT DISTINCT
                picture_id
            FROM  pictures_faces
            WHERE person_id == ?1",
        )?;

        let result: Vec<PictureId> = stmt
            .query_map([person_id.id()], |row| {
                row.get("picture_id").map(PictureId::new)
            })?
            .flatten()
            .collect();

        Ok(result)
    }

    // FIXME probably need a mechanism to undo this in the likely event of user error.
    pub fn mark_ignore(&mut self, face_id: FaceId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    is_ignored = TRUE,
                    is_confirmed = FALSE,
                    is_thumbnail = FALSE,
                    person_id = NULL
                WHERE face_id = ?1",
            )?;

            stmt.execute(params![face_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn mark_face_scan_broken(&mut self, picture_id: &PictureId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO pictures_face_scans (
                    picture_id,
                    is_broken,
                    face_count,
                    scan_ts
                ) VALUES (
                    ?1, TRUE, 0, CURRENT_TIMESTAMP
                ) ON CONFLICT (picture_id) DO UPDATE SET
                    is_broken = true,
                    face_count = 0,
                    scan_ts = CURRENT_TIMESTAMP
                ",
            )?;

            stmt.execute(params![picture_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn add_face_scans(
        &mut self,
        picture_id: &PictureId,
        faces: &Vec<face_extractor::Face>,
    ) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut scan_insert_stmt = tx.prepare_cached(
                "INSERT INTO pictures_face_scans (
                    picture_id,
                    is_broken,
                    face_count,
                    scan_ts
                ) VALUES (
                    ?1, ?2, ?3, CURRENT_TIMESTAMP
                ) ON CONFLICT (picture_id) DO UPDATE SET
                    is_broken = ?2,
                    face_count = ?3,
                    scan_ts = CURRENT_TIMESTAMP
                ",
            )?;

            scan_insert_stmt.execute(params![picture_id.id(), false, faces.len(),])?;

            let mut face_insert_stmt = tx.prepare_cached(
                "INSERT INTO pictures_faces (
                    picture_id,
                    thumbnail_path,
                    bounds_path,

                    model_name,

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

                    confidence,

                    is_ignored
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, false
                )
                ",
            )?;

            for face in faces {
                // convert to relative path before saving to database
                let thumbnail_path = face.thumbnail_path.strip_prefix(&self.data_dir_base_path)?;
                let bounds_path = face.bounds_path.strip_prefix(&self.data_dir_base_path)?;

                let right_eye = face.right_eye();
                let left_eye = face.left_eye();
                let nose = face.nose();
                let right_mouth_corner = face.right_mouth_corner();
                let left_mouth_corner = face.left_mouth_corner();

                face_insert_stmt.execute(params![
                    picture_id.id(),
                    thumbnail_path.to_string_lossy(),
                    bounds_path.to_string_lossy(),
                    face.model_name,
                    face.bounds.x,
                    face.bounds.y,
                    face.bounds.width,
                    face.bounds.height,
                    right_eye.map(|x| x.0),
                    right_eye.map(|x| x.1),
                    left_eye.map(|x| x.0),
                    left_eye.map(|x| x.1),
                    nose.map(|x| x.0),
                    nose.map(|x| x.1),
                    right_mouth_corner.map(|x| x.0),
                    right_mouth_corner.map(|x| x.1),
                    left_mouth_corner.map(|x| x.0),
                    left_mouth_corner.map(|x| x.1),
                    face.confidence
                ])?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    /// Add a new named person derived from a face.
    pub fn add_person(&mut self, face_id: FaceId, name: &str) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            // GTK allows the text gtk::Entry input box to be activated multiple times
            // which results in duplicate people being created :-(
            // FIXME can we configure gtk::Entry to only be activatible once?
            let mut insert_person = tx.prepare_cached(
                "
                INSERT INTO people (name)
                SELECT ?1 AS name
                FROM pictures_faces
                WHERE face_id = ?2 AND person_id IS NULL
                ",
            )?;

            insert_person.execute(params![name, face_id.id(),])?;

            // Zero if no rows inserted.
            // See https://www.sqlite.org/c3ref/last_insert_rowid.html
            let person_id = tx.last_insert_rowid();
            if person_id == 0 {
                warn!("Detected double insert of person. Skipping.");
                return Ok(());
            }

            let mut update_face = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    person_id = ?2,
                    is_confirmed = TRUE,
                    is_thumbnail = TRUE
                WHERE face_id = ?1",
            )?;

            update_face.execute(params![face_id.id(), person_id,])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// User is manually marking a face as a person
    pub fn mark_as_person(&mut self, face_id: FaceId, person_id: PersonId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    person_id = ?2,
                    is_confirmed = TRUE
                WHERE face_id = ?1",
            )?;

            stmt.execute(params![face_id.id(), person_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Face recognition is automatically marking a face as a person
    pub fn mark_as_person_unconfirmed(
        &mut self,
        face_id: FaceId,
        person_id: PersonId,
    ) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    person_id = ?2,
                    is_confirmed = FALSE,
                    is_thumbnail = FALSE
                WHERE face_id = ?1",
            )?;

            stmt.execute(params![face_id.id(), person_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn mark_face_recognition_complete(&mut self, person_id: PersonId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE people
                SET
                    recognized_at = CURRENT_TIMESTAMP
                WHERE person_id = ?1",
            )?;

            stmt.execute(params![person_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn mark_not_person(&mut self, face_id: FaceId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    person_id = NULL,
                    is_confirmed = FALSE,
                    is_thumbnail = FALSE
                WHERE face_id = ?1",
            )?;

            stmt.execute(params![face_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn set_person_thumbnail(&mut self, person_id: PersonId, face_id: FaceId) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    is_thumbnail = FALSE
                WHERE
                    person_id = ?1
                    AND face_id != ?2",
            )?;

            stmt.execute(params![person_id.id(), face_id.id(),])?;

            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    is_confirmed = TRUE,
                    is_thumbnail = TRUE
                WHERE face_id = ?1",
            )?;

            stmt.execute(params![face_id.id(),])?;
        }

        tx.commit()?;
        Ok(())
    }

    fn to_face_and_person(
        &self,
        row: &Row<'_>,
    ) -> rusqlite::Result<(model::Face, Option<model::Person>)> {
        let face_id = row.get("face_id").map(FaceId::new)?;

        let face_thumbnail_path = row
            .get("face_thumbnail_path")
            .map(|p: String| self.data_dir_base_path.join(p))?;

        let face = model::Face {
            face_id,
            thumbnail_path: face_thumbnail_path,
        };

        let person_id = row.get("person_id").map(PersonId::new).ok();

        let person_name = row.get("person_name").ok();

        let small_thumbnail_path = row
            .get("person_thumbnail_path")
            .map(|p: String| self.data_dir_base_path.join(p))
            .ok()
            .expect("Must have small thumbnail path");

        // FIXME should this path be in database?
        let large_thumbnail_path = self
            .cache_dir_base_path
            .join("face_thumbnails")
            .join("large")
            .join(
                small_thumbnail_path
                    .file_name()
                    .expect("Must have file name"),
            );

        let person = if let (Some(person_id), Some(name)) = (person_id, person_name) {
            Some(model::Person {
                person_id,
                name,
                small_thumbnail_path: Some(small_thumbnail_path),
                large_thumbnail_path: Some(large_thumbnail_path),
            })
        } else {
            None
        };

        std::result::Result::Ok((face, person))
    }

    fn to_person(&self, row: &Row<'_>) -> rusqlite::Result<model::Person> {
        let person_id = row.get("person_id").map(PersonId::new)?;

        let name = row.get("person_name")?;

        let small_thumbnail_path = row
            .get("person_thumbnail_path")
            .map(|p: String| self.data_dir_base_path.join(p))
            .ok()
            .expect("Must have small thumbnail path");

        // FIXME should this path be in database?
        let large_thumbnail_path = self
            .cache_dir_base_path
            .join("face_thumbnails")
            .join("large")
            .join(
                small_thumbnail_path
                    .file_name()
                    .expect("Must have file name"),
            );

        std::result::Result::Ok(model::Person {
            person_id,
            name,
            small_thumbnail_path: Some(small_thumbnail_path),
            large_thumbnail_path: Some(large_thumbnail_path),
        })
    }

    fn to_detected_face(&self, row: &Row<'_>) -> rusqlite::Result<model::DetectedFace> {
        let face_id = row.get("face_id").map(FaceId::new)?;

        let face_path = row
            .get("bounds_path")
            .map(|p: String| self.data_dir_base_path.join(p))?;

        let thumbnail_path = row
            .get("thumbnail_path")
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

        let face = model::DetectedFace {
            face_id,
            face_path,
            small_thumbnail_path: thumbnail_path,
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

    fn to_person_for_recognition(
        &self,
        row: &Row<'_>,
    ) -> rusqlite::Result<model::PersonForRecognition> {
        let person_id = row.get("person_id").map(PersonId::new)?;
        let recognized_at = row.get("recognized_at")?;
        let face = self.to_detected_face(row)?;

        let person = PersonForRecognition {
            person_id,
            recognized_at,
            face,
        };

        std::result::Result::Ok(person)
    }

    pub fn migrate_get_all(&self) -> Result<Vec<FaceToMigrate>> {
        let con = self.con.lock().unwrap();
        let mut stmt = con.prepare(
            "SELECT
                migrate_faces.face_id AS face_id,
                migrate_faces.face_index AS face_index,
                pictures.picture_path_b64 AS picture_path_b64,
                pictures_faces.bounds_path AS bounds_path,
                pictures_faces.thumbnail_path AS thumbnail_path
            FROM migrate_faces
            INNER JOIN pictures_faces USING (face_id)
            INNER JOIN pictures USING (picture_id)",
        )?;

        let result: Vec<model::FaceToMigrate> = stmt
            .query_map([], |row| self.to_face_to_migrate(row))?
            .flatten()
            .collect();

        Ok(result)
    }

    pub fn migrate_update_face_paths(&mut self, mf: MigratedFace) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;

        {
            let mut stmt = tx.prepare_cached(
                "UPDATE pictures_faces
                SET
                    bounds_path = ?1,
                    thumbnail_path = ?2
                WHERE face_id = ?3",
            )?;

            stmt.execute(params![
                mf.bounds_path.to_string_lossy(),
                mf.thumbnail_path.to_string_lossy(),
                mf.face_id.id(),
            ])?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn migrate_truncate(&mut self) -> Result<()> {
        let mut con = self.con.lock().unwrap();
        let tx = con.transaction()?;
        {
            tx.execute("DELETE FROM migrate_faces", [])?;
        }
        tx.commit()?;
        Ok(())
    }

    fn to_face_to_migrate(&self, row: &Row<'_>) -> rusqlite::Result<FaceToMigrate> {
        let face_id = row.get("face_id").map(FaceId::new)?;
        let face_index: u32 = row.get("face_index")?;

        let picture_relative_path = row
            .get("picture_path_b64")
            .ok()
            .and_then(|x: String| path_encoding::from_base64(&x).ok())
            .expect("Must have picture path");

        let bounds_path = row
            .get("bounds_path")
            .map(|p: String| self.data_dir_base_path.join(p))?;

        let thumbnail_path = row
            .get("thumbnail_path")
            .map(|p: String| self.data_dir_base_path.join(p))?;

        let face = model::FaceToMigrate {
            face_id,
            face_index,
            picture_relative_path,
            bounds_path,
            thumbnail_path,
        };

        std::result::Result::Ok(face)
    }
}
