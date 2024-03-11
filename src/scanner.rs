use std::path::Path;
use std::path::PathBuf;
use std::{env, fs};

pub struct Scanner {
    scan_path: PathBuf,
}

impl Scanner {
    pub fn build(scan_path: &Path) -> Result<Scanner, String> {
        fs::create_dir_all(scan_path).map_err(|e| e.to_string())?;
        let scan_path = PathBuf::from(scan_path);
        Ok(Scanner { scan_path })
    }
}
