use std::path::Path;
use std::path::PathBuf;
use std::{env, fs};
use walkdir::{DirEntry, WalkDir};

pub struct Scanner {
    scan_path: PathBuf,
}

impl Scanner {
    pub fn build(scan_path: &Path) -> Result<Scanner, String> {
        fs::create_dir_all(scan_path).map_err(|e| e.to_string())?;
        let scan_path = PathBuf::from(scan_path);
        Ok(Scanner { scan_path })
    }

    pub fn scan(&self) {
        WalkDir::new(&self.scan_path)
            .into_iter()
            .for_each(|x| println!("{}", x.unwrap().path().display()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_build() {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        println!("{}", test_data_dir.display());

        let s = Scanner::build(&test_data_dir).unwrap();
        s.scan();
    }
}
