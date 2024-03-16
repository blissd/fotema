///! Repository of metadata about pictures on the local filesystem.
use crate::model::PictureInfo;
use crate::Error::*;
use crate::Result;
use rusqlite::Connection;
use rusqlite::Row;
use std::path;

/// Repository of picture metadata.
/// Repository is backed by a Sqlite database.
pub struct Repository {
    /// Connection to backing Sqlite database.
    con: rusqlite::Connection,
}

impl Repository {
    /// Builds a Repository and creates operational tables.
    pub fn build(_dir_ignored_while_in_memory: &path::Path) -> Result<Repository> {
        let con = Connection::open_in_memory().map_err(|e| RepositoryError(e.to_string()))?;
        let repo = Repository { con };
        repo.setup().map(|_| repo)
    }

    /// Creates operational tables.
    fn setup(&self) -> Result<()> {
        let sql = "CREATE TABLE IF NOT EXISTS PICTURES (
                        path           TEXT PRIMARY KEY UNIQUE ON CONFLICT REPLACE,
                        order_by_ts    TEXT, -- UTC timestamp to order images by
                        fs_modified_at TEXT, -- UTC timestamp of filesystem modification date
                        modified_at    TEXT, -- EXIF time offset-aware timestamp of last modification
                        created_at     TEXT, -- EXIF time offset-aware timestamp of creation
                        description    TEXT  -- EXIF description
                        )";

        let result = self.con.execute(&sql, ());
        result
            .map(|_| ())
            .map_err(|e| RepositoryError(e.to_string()))
    }

    /// Add a picture to the repository.
    /// At a minimum a picture must have a path on the file system and file modification date.
    pub fn add(&self, pic: &PictureInfo) -> Result<()> {
        // Pictures are orderd by this UTC date time.
        let order_by_ts = pic.modified_at.map(|d| d.to_utc()).or(pic.fs_modified_at);

        let result = self.con.execute(
            "INSERT INTO PICTURES (
                path, order_by_ts, fs_modified_at, modified_at, created_at, description
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &pic.path.as_path().to_str(),
                order_by_ts,
                &pic.fs_modified_at,
                &pic.modified_at,
                &pic.created_at,
                &pic.description,
            ),
        );

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(RepositoryError(e.to_string())),
        }
    }

    /// Gets all pictures in the repository, in ascending order of modification timestamp.
    pub fn all(&self) -> Result<Vec<PictureInfo>> {
        let mut stmt = self
            .con
            .prepare(
                "SELECT 
            path,
            fs_modified_at,
            modified_at, 
            created_at, 
            description
            from PICTURES order by order_by_ts ASC",
            )
            .unwrap();

        fn row_to_picture_info(row: &Row<'_>) -> rusqlite::Result<PictureInfo> {
            let path_result: rusqlite::Result<String> = row.get(0);
            if let Some(path) = path_result.ok() {
                Ok(PictureInfo {
                    path: path::PathBuf::from(path),
                    // order_by_ts: row.get(0).ok(),
                    fs_modified_at: row.get(1).ok(),
                    modified_at: row.get(2).ok(),
                    created_at: row.get(3).ok(),
                    description: row.get(4).ok(),
                })
            } else {
                Err(rusqlite::Error::ExecuteReturnedResults) // probably not the right error
            }
        }
        let iter = match stmt.query_map([], |row| row_to_picture_info(row)) {
            Ok(f) => f,
            Err(e) => return Err(RepositoryError(e.to_string())),
        };

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

    fn picture_dir() -> PathBuf {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        test_data_dir
    }

    #[test]
    fn repo_add_and_get() {
        let r = Repository::build(path::Path::new(":memory:")).unwrap();

        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Birdie.jpg");

        let pic = PictureInfo::new(test_file.clone());
        r.add(&pic).unwrap();

        let all_pics = r.all().unwrap();
        let pic = all_pics.get(0).unwrap();
        assert_eq!(pic.path, test_file);
    }
}
