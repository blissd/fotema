use sqlite;
use std::path;

pub struct PicturesRepo {
    con: sqlite::Connection,
}

impl PicturesRepo {
    pub fn build(dir: &path::Path) -> Result<PicturesRepo, String> {
        let con = sqlite::open(dir);

        con.and_then(|con| {
            let sql = "create table if not exists pictures (
                        path text primary key unique on conflict replace,
                        fs_modified_at   text,
                        modified_at text,
                        created_at text,
                        description text
                        )
                        ";
            let result = con.execute(&sql);
            result.map(|x| PicturesRepo { con })
        })
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
