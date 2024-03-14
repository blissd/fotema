use crate::model::PictureInfo;
use crate::Error::*;
use crate::Result;
use rusqlite::Connection;
use std::path;

pub struct Repository {
    con: rusqlite::Connection,
}

impl Repository {
    pub fn build(_dir_ignored_while_in_memory: &path::Path) -> Result<Repository > {
        let con = Connection::open_in_memory().map_err(|e| RepositoryError(e.to_string()))?;
        let repo = Repository { con };
        repo.setup().map(|_| repo)
    }

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
        result.map(|_| ()).map_err(|e| RepositoryError(e.to_string()))
    }

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

        let pic = PictureInfo::new(test_file);
        r.add(&pic).unwrap();
    }
}
