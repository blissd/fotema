// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::scanner;
///! Repository of metadata about pictures on the local filesystem.
use crate::Error::*;
use crate::Result;
use crate::YearMonth;
use chrono::*;
use rusqlite::params;
use rusqlite::Batch;
use rusqlite::Connection;
use rusqlite::Error::SqliteFailure;
use rusqlite::ErrorCode::ConstraintViolation;
use std::fmt::Display;
use std::path;
use std::path::PathBuf;

/// Database ID of picture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureId(i64);

impl PictureId {
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl Display for PictureId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A picture in the repository
#[derive(Debug, Clone)]
pub struct Picture {
    /// Full path from picture library root.
    pub path: PathBuf,

    /// Database primary key for picture
    pub picture_id: PictureId,

    /// Full path to square preview image
    pub square_preview_path: Option<PathBuf>,

    /// Ordering timestamp, derived from EXIF metadata or file system timestamps.
    pub order_by_ts: Option<DateTime<Utc>>,

    /// Was picture taken with front camera?
    pub is_selfie: bool,
}

impl Picture {
    pub fn year(&self) -> u32 {
        self.order_by_ts
            .map(|ts| ts.date_naive().year_ce().1)
            .unwrap_or(0)
    }

    pub fn year_month(&self) -> YearMonth {
        self.order_by_ts
            .map(|ts| {
                let year = ts.date_naive().year();
                let month = ts.date_naive().month();
                let month = chrono::Month::try_from(u8::try_from(month).unwrap()).unwrap();
                YearMonth { year, month }
            })
            .unwrap_or(YearMonth {
                year: 0,
                month: chrono::Month::January,
            })
    }

    pub fn date(&self) -> Option<chrono::NaiveDate> {
        self.order_by_ts.map(|ts| ts.date_naive())
    }
}

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug)]
pub struct Repository {
    /// Base path to picture library on file system
    library_base_path: path::PathBuf,

    /// Connection to backing Sqlite database.
    con: rusqlite::Connection,
}

impl Repository {
    pub fn open_in_memory(library_base_path: &path::Path) -> Result<Repository> {
        let con = Connection::open_in_memory().map_err(|e| RepositoryError(e.to_string()))?;
        let library_base_path = path::PathBuf::from(library_base_path);
        let repo = Repository {
            library_base_path,
            con,
        };
        repo.setup().map(|_| repo)
    }

    /// Builds a Repository and creates operational tables.
    pub fn open(library_base_path: &path::Path, db_path: &path::Path) -> Result<Repository> {
        let con = Connection::open(db_path).map_err(|e| RepositoryError(e.to_string()))?;
        let library_base_path = path::PathBuf::from(library_base_path);
        let repo = Repository {
            library_base_path,
            con,
        };
        repo.setup().map(|_| repo)
    }

    /// Creates operational tables.
    fn setup(&self) -> Result<()> {
        if !self.library_base_path.is_dir() {
            return Err(RepositoryError(format!(
                "{:?} is not a directory",
                self.library_base_path
            )));
        }

        let sql = vec![
            "CREATE TABLE IF NOT EXISTS pictures (
                picture_id     INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for picture
                relative_path  TEXT UNIQUE NOT NULL ON CONFLICT IGNORE,
                order_by_ts    DATETIME, -- UTC timestamp to order images by
                is_selfie      BOOLEAN NOT NULL CHECK (is_selfie IN (0, 1)) -- front camera?
            )",
            "CREATE TABLE IF NOT EXISTS previews (
                preview_id INTEGER PRIMARY KEY UNIQUE NOT NULL, -- pk for preview
                picture_id INTEGER UNIQUE NOT NULL ON CONFLICT IGNORE, -- fk to pictures. Only one preview allowed for now.
                full_path  TEXT UNIQUE NOT NULL ON CONFLICT IGNORE, -- full path to preview image
                FOREIGN KEY (picture_id) REFERENCES pictures (picture_id)
            )",
        ];

        let sql = sql.join(";\n") + ";";

        let mut batch = Batch::new(&self.con, &sql);
        while let Some(mut stmt) = batch.next().map_err(|e| RepositoryError(e.to_string()))? {
            stmt.execute([])
                .map_err(|e| RepositoryError(e.to_string()))?;
        }

        let result = self.con.execute(&sql, ());
        result
            .map(|_| ())
            .map_err(|e| RepositoryError(e.to_string()))
    }

    pub fn add_preview(&mut self, pic: &Picture) -> Result<()> {
        let tx = self
            .con
            .transaction()
            .map_err(|e| RepositoryError(e.to_string()))?;

        {
            let mut stmt = tx
                .prepare("INSERT INTO previews (picture_id, full_path) VALUES (?1, ?2)")
                .map_err(|e| RepositoryError(e.to_string()))?;

            let result = stmt.execute(params![
                pic.picture_id.0,
                pic.square_preview_path.as_ref().map(|p| p.to_str()),
            ]);

            //result
            //  .map(|_| ())
            //.map_err(|e| RepositoryError(e.to_string()))?;

            // The "on conflict ignore" constraints look like errors to rusqlite
            match result {
                Err(e @ SqliteFailure(_, _))
                    if e.sqlite_error_code() == Some(ConstraintViolation) =>
                {
                    // println!("Skipping {:?} {}", path, e);
                }
                other => {
                    other.map_err(|e| RepositoryError(format!("Preview: {}", e)))?;
                }
            }
        }

        tx.commit().map_err(|e| RepositoryError(e.to_string()))
    }

    /// Add all Pictures received from a vector.
    pub fn add_all(&mut self, pics: &Vec<scanner::Picture>) -> Result<()> {
        let tx = self
            .con
            .transaction()
            .map_err(|e| RepositoryError(format!("Starting transaction: {}", e)))?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut stmt = tx
                .prepare_cached(
                    "INSERT INTO pictures (
                    relative_path,
                    order_by_ts,
                    is_selfie
                ) VALUES (?1, ?2, ?3)",
                )
                .map_err(|e| RepositoryError(format!("Preparing statement: {}", e)))?;

            for pic in pics {
                // convert to relative path before saving to database
                let path = pic
                    .path
                    .strip_prefix(&self.library_base_path)
                    .map_err(|e| {
                        RepositoryError(format!("Stripping prefix for {:?}: {}", &pic.path, e))
                    })?;

                let exif_date_time = pic.exif.as_ref().and_then(|x| x.created_at);
                let fs_date_time = pic.fs.as_ref().and_then(|x| x.created_at);
                let order_by_ts = exif_date_time.map(|d| d.to_utc()).or(fs_date_time);
                let is_selfie = pic
                    .exif
                    .as_ref()
                    .and_then(|x| x.lens_model.as_ref())
                    .is_some_and(|x| x.contains("front"));

                let result = stmt.execute(params![path.to_str(), order_by_ts, is_selfie]);

                // The "on conflict ignore" constraints look like errors to rusqlite
                match result {
                    Err(e @ SqliteFailure(_, _))
                        if e.sqlite_error_code() == Some(ConstraintViolation) =>
                    {
                        // println!("Skipping {:?} {}", path, e);
                    }
                    other => {
                        other.map_err(|e| RepositoryError(format!("Inserting: {}", e)))?;
                    }
                }
            }
        }

        tx.commit()
            .map_err(|e| RepositoryError(format!("Committing transaction: {}", e)))
    }

    /// Gets all pictures in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Picture>> {
        let mut stmt = self
            .con
            .prepare(
                "SELECT
                    pictures.picture_id,
                    pictures.relative_path,
                    previews.full_path as square_preview_path,
                    pictures.order_by_ts,
                    pictures.is_selfie
                FROM pictures
                LEFT JOIN previews ON pictures.picture_id = previews.picture_id
                ORDER BY order_by_ts ASC",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Picture {
                    picture_id: PictureId(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    square_preview_path: row.get(2).ok().map(|p: String| path::PathBuf::from(p)),
                    order_by_ts: row.get(3).ok(),
                    is_selfie: row.get(4).ok().unwrap_or(false),
                })
            })
            .map_err(|e| RepositoryError(e.to_string()))?;

        // Would like to return an iterator... but Rust is defeating me.
        let mut pics = Vec::new();
        for pic in iter.flatten() {
            pics.push(pic);
        }

        Ok(pics)
    }

    pub fn get(&mut self, picture_id: PictureId) -> Result<Option<Picture>> {
        let mut stmt = self
            .con
            .prepare(
                "SELECT
                    pictures.picture_id,
                    pictures.relative_path,
                    previews.full_path as square_preview_path,
                    pictures.order_by_ts,
                    pictures.is_selfie
                FROM pictures
                LEFT JOIN previews ON pictures.picture_id = previews.picture_id
                WHERE pictures.picture_id = ?1",
            )
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([picture_id.0], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Picture {
                    picture_id: PictureId(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    square_preview_path: row.get(2).ok().map(|p: String| path::PathBuf::from(p)),
                    order_by_ts: row.get(3).ok(),
                    is_selfie: row.get(4).ok().unwrap_or(false),
                })
            })
            .map_err(|e| RepositoryError(e.to_string()))?;

        let head = iter.flatten().nth(0);
        Ok(head)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn repo_add_and_get() {
        let mut r = Repository::open_in_memory(path::Path::new("/var/empty")).unwrap();
        let test_file = PathBuf::from("/var/empty/some/random/path.jpg");
        let pic = scanner::Picture::new(test_file.clone());
        let pics = vec![pic];
        r.add_all(&pics).unwrap();

        let all_pics = r.all().unwrap();
        let pic = all_pics.get(0).unwrap();
        assert_eq!(pic.path, test_file);
    }
}
