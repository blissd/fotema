use sqlite;
use std::path;

struct PicturesRepo {
    con: sqlite::Connection,
}

impl PicturesRepo {
    fn build(dir: &path::Path) -> Result<PicturesRepo, String> {
        match sqlite::open(dir) {
            Ok(con) => {
                let r = PicturesRepo { con };
                Ok(r)
            }
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
