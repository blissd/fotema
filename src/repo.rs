// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

///! Repository of metadata about pictures on the local filesystem.
use crate::Error::*;
use crate::Result;
use chrono::*;
use rusqlite::Connection;
use rusqlite::Row;
use std::path;
use std::path::PathBuf;

/// A picture in the repository
#[derive(Debug)]
pub struct Picture {
    /// Relative path from picture library root.
    pub relative_path: PathBuf,

    /// Ordering timestamp, derived from EXIF metadata or file system timestamps.
    pub order_by_ts: Option<DateTime<Utc>>,
}

impl Picture {
    pub fn new(relative_path: PathBuf) -> Picture {
        Picture {
            relative_path,
            order_by_ts: None,
        }
    }
}

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
pub struct Repository {
    /// Connection to backing Sqlite database.
    con: rusqlite::Connection,
}

impl Repository {
    pub fn open_in_memory() -> Result<Repository> {
        let con = Connection::open_in_memory().map_err(|e| RepositoryError(e.to_string()))?;
        let repo = Repository { con };
        repo.setup().map(|_| repo)
    }

    /// Builds a Repository and creates operational tables.
    pub fn open(db_path: &path::Path) -> Result<Repository> {
        let con = Connection::open(db_path).map_err(|e| RepositoryError(e.to_string()))?;
        let repo = Repository { con };
        repo.setup().map(|_| repo)
    }

    /// Creates operational tables.
    fn setup(&self) -> Result<()> {
        let sql = "CREATE TABLE IF NOT EXISTS PICTURES (
                        relative_path  TEXT PRIMARY KEY UNIQUE ON CONFLICT REPLACE,
                        order_by_ts    DATETIME -- UTC timestamp to order images by
                        )";

        let result = self.con.execute(&sql, ());
        result
            .map(|_| ())
            .map_err(|e| RepositoryError(e.to_string()))
    }

    /// Add a picture to the repository.
    /// At a minimum a picture must have a path on the file system and file modification date.
    pub fn add(&self, pic: &Picture) -> Result<()> {
        let result = self.con.execute(
            "INSERT INTO PICTURES (relative_path, order_by_ts) VALUES (?1, ?2)",
            (pic.relative_path.as_path().to_str(), pic.order_by_ts),
        );

        result
            .map(|_| ())
            .map_err(|e| RepositoryError(e.to_string()))
    }

    /// Gets all pictures in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<Picture>> {
        let mut stmt = self
            .con
            .prepare("SELECT relative_path, order_by_ts from PICTURES order by order_by_ts ASC")
            .unwrap();

        fn row_to_picture(row: &Row<'_>) -> rusqlite::Result<Picture> {
            let path_result: rusqlite::Result<String> = row.get(0);
            path_result.map(|relative_path| Picture {
                relative_path: path::PathBuf::from(relative_path),
                order_by_ts: row.get(1).ok(),
            })
        }

        let iter = stmt
            .query_map([], |row| row_to_picture(row))
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
        let r = Repository::open_in_memory().unwrap();
        let test_file = PathBuf::from("some/random/path.jpg");
        let pic = Picture::new(test_file.clone());
        r.add(&pic).unwrap();

        let all_pics = r.all().unwrap();
        let pic = all_pics.get(0).unwrap();
        assert_eq!(pic.relative_path, test_file);
    }
}
