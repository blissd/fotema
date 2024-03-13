use chrono;
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

    pub fn visit_all<F>(&self, func: F)
    where
        F: FnMut(PictureInfo),
    {
        let picture_suffixes = vec![
            String::from("avif"),
            String::from("heic"),
            String::from("jpeg"),
            String::from("jpg"),
            String::from("png"),
            String::from("tiff"),
            String::from("webp"),
        ];

        WalkDir::new(&self.scan_path)
            .into_iter()
            .flatten() // skip files we failed to read
            .filter(|x| x.path().is_file()) // only process files
            .filter(|x| {
                // only process supported image types
                let ext = x
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_lowercase());
                picture_suffixes.contains(&ext.unwrap_or(String::from("not_an_image")))
            })
            .map(|x| self.scan_one(x.path())) // Get picture info for image path
            .flatten() // ignore any errors when reading images
            .for_each(func); // visit
    }

    pub fn scan_one(&self, path: &Path) -> Result<PictureInfo, String> {
        let f = match fs::File::open(path) {
            Ok(file) => file,
            Err(e) => {
                println!("file {:?} failed with {}", path, e);
                return Result::Err(e.to_string());
            }
        };

        let f = &mut BufReader::new(f);
        let r = match exif::Reader::new().read_from_container(f) {
            Ok(file) => file,
            Err(e) => {
                println!("reader for {:?} failed with {}", path, e);
                return Result::Err(e.to_string());
            }
        };

        let width = r
            .get_field(exif::Tag::PixelXDimension, exif::In::PRIMARY)
            .and_then(|e| e.value.get_uint(0));

        let height = r
            .get_field(exif::Tag::PixelYDimension, exif::In::PRIMARY)
            .and_then(|e| e.value.get_uint(0));

        let description = r
            .get_field(exif::Tag::ImageDescription, exif::In::PRIMARY)
            .map(|e| e.display_value().to_string());

        fn parse_date_time(date_time_field: Option<&exif::Field>) -> Option<chrono::NaiveDateTime> {
            let date_time = if let Some(field) = date_time_field {
                match field.value {
                    exif::Value::Ascii(ref vec) if !vec.is_empty() => {
                        exif::DateTime::from_ascii(&vec[0]).ok()
                    }
                    _ => None,
                }
            } else {
                None
            };

            let date_time = date_time.and_then(|x| {
                let date =
                    chrono::NaiveDate::from_ymd_opt(x.year.into(), x.month.into(), x.day.into());
                let time = chrono::NaiveTime::from_hms_opt(
                    x.hour.into(),
                    x.minute.into(),
                    x.second.into(),
                );
                // convert to a NaiveDateTime without a time zone
                let dt = date.and_then(|d| time.map(|t| d.and_time(t)));
                dt
            });

            date_time
        }

        // TODO offsets are in separate fields to timestamps
        //let created_at_offset = r.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY);
        let created_at =
            parse_date_time(r.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY));
        let modified_at = parse_date_time(r.get_field(exif::Tag::DateTime, exif::In::PRIMARY));

        Ok(PictureInfo {
            path: PathBuf::from(path),
            width,
            height,
            description,
            created_at,
            modified_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture_dir() -> PathBuf {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        test_data_dir
    }

    #[test]
    fn visit_all() {
        //let test_data_dir = picture_dir();
        let test_data_dir = PathBuf::from("/var/home/david/Pictures");
        let s = Scanner::build(&test_data_dir).unwrap();
        s.visit_all(|x| println!("{:?}", x));
    }

    #[test]
    fn scan_one() {
        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Birdie.jpg");

        let s = Scanner::build(&test_data_dir).unwrap();
        let info = s.scan_one(&test_file).unwrap();
        println!("{:?}", info);
    }
}
