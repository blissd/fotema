use exif;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

use crate::model::PictureInfo;

pub struct Scanner {
    scan_path: PathBuf,
}

impl Scanner {
    pub fn build(scan_path: &Path) -> Result<Scanner, String> {
        fs::create_dir_all(scan_path).map_err(|e| e.to_string())?;
        let scan_path = PathBuf::from(scan_path);
        Ok(Scanner { scan_path })
    }

    pub fn scan_all(&self) {
        WalkDir::new(&self.scan_path)
            .into_iter()
            .for_each(|x| println!("{}", x.unwrap().path().display()));
    }

    pub fn scan_one(&self, path: &Path) -> Result<PictureInfo, String> {
        let f = fs::File::open(path).unwrap();
        let f = &mut BufReader::new(f);
        let r = exif::Reader::new().read_from_container(f).unwrap();

        let width = r
            .get_field(exif::Tag::PixelXDimension, exif::In::PRIMARY)
            .and_then(|e| e.value.get_uint(0));

        let height = r
            .get_field(exif::Tag::PixelYDimension, exif::In::PRIMARY)
            .and_then(|e| e.value.get_uint(0));

        Ok(PictureInfo {
            path: PathBuf::from(path),
            width,
            height,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_all() {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        println!("{}", test_data_dir.display());

        let s = Scanner::build(&test_data_dir).unwrap();
        s.scan_all();
    }

    #[test]
    fn scan_one() {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");

        let mut test_file = test_data_dir.clone();
        test_file.push("Birdie.jpg");

        let s = Scanner::build(&test_data_dir).unwrap();
        let info = s.scan_one(&test_file).unwrap();
        println!("{:?}", info);
    }
}
