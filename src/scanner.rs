use crate::Error::*;
use crate::Result;
use chrono::prelude::*;
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
    pub fn build(scan_path: &Path) -> Result<Scanner> {
        fs::create_dir_all(scan_path).map_err(|e| ScannerError(e.to_string()))?;
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

    pub fn scan_one(&self, path: &Path) -> Result<PictureInfo> {
        let f = match fs::File::open(path) {
            Ok(file) => file,
            Err(e) => {
                println!("file {:?} failed with {}", path, e);
                return Err(ScannerError(e.to_string()));
            }
        };

        let mut pic = PictureInfo::new(PathBuf::from(path));
        let fs_modified_at = f.metadata().ok().and_then(|x| x.modified().ok());
        pic.fs_modified_at = fs_modified_at.map(|x| {
            let dt: DateTime<Utc> = x.into();
            dt
        });

        let f = &mut BufReader::new(f);
        let r = match exif::Reader::new().read_from_container(f) {
            Ok(file) => file,
            Err(_) => {
                // Assume this error is when there is no EXIF data.
                return Ok(pic);
            }
        };

        pic.description = r
            .get_field(exif::Tag::ImageDescription, exif::In::PRIMARY)
            .map(|e| e.display_value().to_string());

        fn parse_date_time(
            date_time_field: Option<&exif::Field>,
            time_offset_field: Option<&exif::Field>,
        ) -> Option<DateTime<FixedOffset>> {
            let date_time_field = date_time_field?;
            //let time_offset_field = time_offset_field?;

            let mut date_time = match date_time_field.value {
                exif::Value::Ascii(ref vec) => exif::DateTime::from_ascii(&vec[0]).ok(),
                _ => None,
            }?;

            if let Some(field) = time_offset_field {
                if let exif::Value::Ascii(ref vec) = field.value {
                    date_time.parse_offset(&vec[0]);
                }
            }

            let offset = date_time.offset.unwrap_or(0); // offset in minutes
            let offset = FixedOffset::east_opt((offset as i32) * 60)?;

            let date = NaiveDate::from_ymd_opt(
                date_time.year.into(),
                date_time.month.into(),
                date_time.day.into(),
            )?;

            let time = NaiveTime::from_hms_opt(
                date_time.hour.into(),
                date_time.minute.into(),
                date_time.second.into(),
            )?;

            let naive_date_time = date.and_time(time);
            Some(offset.from_utc_datetime(&naive_date_time))
        }

        pic.created_at = parse_date_time(
            r.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY),
            r.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY),
        );
        pic.modified_at = parse_date_time(
            r.get_field(exif::Tag::DateTime, exif::In::PRIMARY),
            r.get_field(exif::Tag::OffsetTime, exif::In::PRIMARY),
        );

        Ok(pic)
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
