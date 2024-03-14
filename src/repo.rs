use crate::model::PictureInfo;
use rusqlite::{Connection, Result};
use std::path;

pub struct PicturesRepo {
    con: rusqlite::Connection,
}

impl PicturesRepo {
    pub fn build(_dir_ignored_while_in_memory: &path::Path) -> Result<PicturesRepo, String> {
        let con = Connection::open_in_memory().map_err(|e| e.to_string())?;

        let sql = "CREATE TABLE IF NOT EXISTS PICTURES (
                        path           TEXT PRIMARY KEY UNIQUE ON CONFLICT REPLACE,
                        fs_modified_at TEXT,
                        modified_at    TEXT,
                        created_at     TEXT,
                        description    TEXT
                        )";

        let result = con.execute(&sql, ());
        result
            .map(|_| PicturesRepo { con })
            .map_err(|e| e.to_string())
    }

    pub fn add(&self, pic: &PictureInfo) -> Result<(), String> {
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
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_repo_build() {
        let r = PicturesRepo::build(path::Path::new(":memory:"));
    }
}
