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
        let con = Connection::open_in_memory().map_err(|e| DatabaseError(e.to_string()))?;
        let repo = Repository { con };
        repo.setup().map(|_| repo)
    }

    fn setup(&self) -> Result<()> {
        let sql = "CREATE TABLE IF NOT EXISTS PICTURES (
                        path           TEXT PRIMARY KEY UNIQUE ON CONFLICT REPLACE,
                        fs_modified_at TEXT,
                        modified_at    TEXT,
                        created_at     TEXT,
                        description    TEXT
                        )";

        let result = self.con.execute(&sql, ());
        result.map(|_| ()).map_err(|e| DatabaseError(e.to_string()))
    }

    pub fn add(&self, pic: &PictureInfo) -> Result<()> {
        let result = self.con.execute(
            "INSERT INTO PICTURES (
                path, fs_modified_at, modified_at, created_at, description
            ) values (?1, ?2, ?3, ?4, ?5)",
            (
                &pic.path.as_path().to_str(),
                &pic.fs_modified_at,
                &pic.modified_at,
                &pic.created_at,
                &pic.description,
            ),
        );

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(DatabaseError(e.to_string())),
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
    fn pictures_repo_build() {
        let r = Repository::build(path::Path::new(":memory:")).unwrap();

        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Birdie.jpg");

        let pic = PictureInfo::new(test_file);
        r.add(&pic).unwrap();
    }
}
