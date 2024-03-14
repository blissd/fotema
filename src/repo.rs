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

    //pub fn add(&self, pic: &PictureInfo) -> Result<(), String> {
    //self.con.execute(kkkkk)
    //}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_repo_build() {
        let r = PicturesRepo::build(path::Path::new(":memory:"));
    }
}
