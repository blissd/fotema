// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

///! Repository of metadata about pictures on the local filesystem.
use crate::Error::*;
use crate::Result;
use chrono::*;
use rusqlite::params;
use rusqlite::Batch;
use rusqlite::Connection;
use std::path;
use std::path::PathBuf;

/// A picture in the repository
#[derive(Debug)]
pub struct Picture {
    /// Full path from picture library root.
    pub path: PathBuf,

    /// Ordering timestamp, derived from EXIF metadata or file system timestamps.
    pub order_by_ts: Option<DateTime<Utc>>,
}

impl Picture {
    pub fn new(path: PathBuf) -> Picture {
        Picture {
            path,
            order_by_ts: None,
        }
    }
}

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
#[derive(Debug)]
pub struct Repository {
    /// Base path to picture library on file system
    base_path: path::PathBuf,

    /// Connection to backing Sqlite database.
    con: rusqlite::Connection,
}

impl Repository {
    pub fn open_in_memory(base_path: &path::Path) -> Result<Repository> {
        let con = Connection::open_in_memory().map_err(|e| RepositoryError(e.to_string()))?;
        let base_path = path::PathBuf::from(base_path);
        let repo = Repository { base_path, con };
        repo.setup().map(|_| repo)
    }

    /// Builds a Repository and creates operational tables.
    pub fn open(base_path: &path::Path, db_path: &path::Path) -> Result<Repository> {
        let con = Connection::open(db_path).map_err(|e| RepositoryError(e.to_string()))?;
        let base_path = path::PathBuf::from(base_path);
        let repo = Repository { base_path, con };
        repo.setup().map(|_| repo)
    }

    /// Creates operational tables.
    fn setup(&self) -> Result<()> {
        if !self.base_path.is_dir() {
            return Err(RepositoryError(format!(
                "{} is not a directory",
                self.base_path.to_string_lossy()
            )));
        }

        let sql = vec![
            "CREATE TABLE IF NOT EXISTS PICTURES (
            picture_id     INTEGER PRIMARY KEY UNIQUE, -- unique ID for picture
            relative_path  TEXT UNIQUE ON CONFLICT IGNORE,
            order_by_ts    DATETIME -- UTC timestamp to order images by
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

    /// Add a picture to the repository.
    /// At a minimum a picture must have a path on the file system and file modification date.
    pub fn add(&self, pic: &Picture) -> Result<()> {
        // convert to relative path before saving to database
        let path = pic
            .path
            .strip_prefix(&self.base_path)
            .map_err(|e| RepositoryError(e.to_string()))?;

        let result = self.con.execute(
            "INSERT INTO PICTURES (relative_path, order_by_ts) VALUES (?1, ?2)",
            (path.to_str(), pic.order_by_ts),
        );

        result
            .map(|_| ())
            .map_err(|e| RepositoryError(e.to_string()))
    }

    /// Add all Pictures received from a vector.
    pub fn add_all(&mut self, pics: &Vec<Picture>) -> Result<()> {
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
                    .strip_prefix(&self.base_path)
                    .map_err(|e| RepositoryError(e.to_string()))?;

                stmt.execute(params![path.to_str(), pic.order_by_ts])
                    .map_err(|e| RepositoryError(e.to_string()))?;
            }
        }

        tx.commit().map_err(|e| RepositoryError(e.to_string()))
    }

    /// Gets all pictures in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Picture>> {
        let mut stmt = self
            .con
            .prepare("SELECT relative_path, order_by_ts from PICTURES order by order_by_ts ASC")
            .unwrap();

        let iter = stmt
            .query_map([], |row| {
                let path_result: rusqlite::Result<String> = row.get(0);
                path_result.map(|relative_path| Picture {
                    path: self.base_path.join(relative_path),
                    order_by_ts: row.get(1).ok(),
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
        let r = Repository::open_in_memory(path::Path::new("/var/empty")).unwrap();
        let test_file = PathBuf::from("/var/empty/some/random/path.jpg");
        let pic = Picture::new(test_file.clone());
        r.add(&pic).unwrap();

        let all_pics = r.all().unwrap();
        let pic = all_pics.get(0).unwrap();
        assert_eq!(pic.path, test_file);
    }
}
