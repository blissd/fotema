// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::scanner;
///! Repository of metadata about pictures on the local filesystem.
use crate::Error::*;
use crate::Result;
use chrono::*;
use rusqlite::params;
use rusqlite::Batch;
use rusqlite::Connection;
use std::fmt::Display;
use std::path;
use std::path::PathBuf;

/// Database ID of picture
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug)]
pub struct Picture {
    /// Full path from picture library root.
    pub path: PathBuf,

    /// Database primary key for picture
    pub picture_id: PictureId,

    /// Full path to square preview image
    pub square_preview_path: Option<PathBuf>,

    /// Ordering timestamp, derived from EXIF metadata or file system timestamps.
    pub order_by_ts: Option<DateTime<Utc>>,
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
                "{} is not a directory",
                self.library_base_path.to_string_lossy()
            )));
        }

        let sql = vec![
            "CREATE TABLE IF NOT EXISTS pictures (
            picture_id     INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for picture
            relative_path  TEXT UNIQUE NOT NULL ON CONFLICT IGNORE,
            square_preview_path TEXT UNIQUE,
            order_by_ts    DATETIME -- UTC timestamp to order images by
            )",
            "CREATE TABLE IF NOT EXISTS previews (
                preview_id INTEGER PRIMARY KEY UNIQUE NOT NULL, -- pk for preview
                picture_id INTEGER NOT NULL, -- fk to pictures
                full_path  TEXT UNIQUE NOT NULL, -- full path to preview image
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
                .prepare("UPDATE PICTURES SET square_preview_path = ?1 WHERE picture_id = ?2")
                .map_err(|e| RepositoryError(e.to_string()))?;

            let result = stmt.execute(params![
                pic.square_preview_path.as_ref().map(|p| p.to_str()),
                pic.picture_id.0,
            ]);

            result
                .map(|_| ())
                .map_err(|e| RepositoryError(e.to_string()))?;
        }

        tx.commit().map_err(|e| RepositoryError(e.to_string()))
    }

    /// Add all Pictures received from a vector.
    pub fn add_all(&mut self, pics: &Vec<scanner::Picture>) -> Result<()> {
        let tx = self
            .con
            .transaction()
            .map_err(|e| RepositoryError(e.to_string()))?;

        // Create a scope to make borrowing of tx not be an error.
        {
            let mut stmt = tx
                .prepare_cached("INSERT INTO PICTURES (relative_path, order_by_ts) VALUES (?1, ?2)")
                .map_err(|e| RepositoryError(e.to_string()))?;

            for pic in pics {
                // convert to relative path before saving to database
                let path = pic
                    .path
                    .strip_prefix(&self.library_base_path)
                    .map_err(|e| RepositoryError(e.to_string()))?;

                let exif_date_time = pic.exif.as_ref().and_then(|x| x.created_at);
                let fs_date_time = pic.fs.as_ref().and_then(|x| x.created_at);
                let order_by_ts = exif_date_time.map(|d| d.to_utc()).or(fs_date_time);

                stmt.execute(params![path.to_str(), order_by_ts])
                    .map_err(|e| RepositoryError(e.to_string()))?;
            }
        }

        tx.commit().map_err(|e| RepositoryError(e.to_string()))
    }

    /// Gets all pictures in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Picture>> {
        let mut stmt = self
            .con
            .prepare("SELECT picture_id, relative_path, square_preview_path, order_by_ts from PICTURES order by order_by_ts ASC")
            .map_err(|e| RepositoryError(e.to_string()))?;

        let iter = stmt
            .query_map([], |row| {
                let path_result: rusqlite::Result<String> = row.get(1);
                path_result.map(|relative_path| Picture {
                    picture_id: PictureId(row.get(0).unwrap()), // should always have a primary key
                    path: self.library_base_path.join(relative_path), // compute full path
                    square_preview_path: row.get(2).ok().map(|p: String| path::PathBuf::from(p)),
                    order_by_ts: row.get(3).ok(),
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
