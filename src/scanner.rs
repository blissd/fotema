// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Error::*;
use crate::Result;
use chrono::prelude::*;
use exif;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

/// A picture on the local file systme that has been scanned.
#[derive(Debug)]
pub struct Picture {
    /// Relative path to file under scanner's scan_path.
    pub relative_path: PathBuf,

    /// Metadata from the file system.
    pub fs: Option<FsMetadata>,

    /// Metadata from the EXIF tags.
    pub exif: Option<Exif>,
}

impl Picture {
    /// Creates a new Picture for a given relative path.
    pub fn new(relative_path: PathBuf) -> Picture {
        Picture {
            relative_path,
            fs: None,
            exif: None,
        }
    }
}

/// Metadata from EXIF tags
#[derive(Debug, Default)]
pub struct Exif {
    pub description: Option<String>,
    pub created_at: Option<DateTime<FixedOffset>>,
    pub modified_at: Option<DateTime<FixedOffset>>,
}

impl Exif {
    /// If any fields are present, then wrap self in an Option::Some. Otherwise, return None.
    pub fn to_option(self) -> Option<Exif> {
        if self.description.is_some() || self.created_at.is_some() || self.modified_at.is_some() {
            Some(self)
        } else {
            None
        }
    }
}

/// Metadata from the file system.
#[derive(Debug, Default)]
pub struct FsMetadata {
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
}

impl FsMetadata {
    /// If any fields are present, then wrap self in an Option::Some. Otherwise, return None.
    pub fn to_option(self) -> Option<FsMetadata> {
        if self.created_at.is_some() || self.modified_at.is_some() {
            Some(self)
        } else {
            None
        }
    }
}

/// Scans a file system for pictures.
pub struct Scanner {
    /// File system path to scan.
    scan_base: PathBuf,
}

impl Scanner {
    pub fn build(scan_base: &Path) -> Result<Scanner> {
        fs::create_dir_all(scan_base).map_err(|e| ScannerError(e.to_string()))?;
        let scan_base = PathBuf::from(scan_base);
        Ok(Scanner { scan_base })
    }

    pub fn visit_all<F>(&self, func: F)
    where
        F: FnMut(Picture),
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

        WalkDir::new(&self.scan_base)
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

    pub fn scan_one(&self, path: &Path) -> Result<Picture> {
        let file = fs::File::open(path).map_err(|e| ScannerError(e.to_string()))?;
        let mut fs = FsMetadata::default();

        fs.created_at = file
            .metadata()
            .ok()
            .and_then(|x| x.created().ok())
            .map(|x| Into::<DateTime<Utc>>::into(x));

        fs.modified_at = file
            .metadata()
            .ok()
            .and_then(|x| x.modified().ok())
            .map(|x| Into::<DateTime<Utc>>::into(x));

        let mut pic = {
            let relative_path = path
                .strip_prefix(&self.scan_base)
                .map_err(|e| ScannerError(e.to_string()))?;
            Picture::new(PathBuf::from(relative_path))
        };

        pic.fs = fs.to_option();

        let exif_data = {
            let f = &mut BufReader::new(file);
            match exif::Reader::new().read_from_container(f) {
                Ok(file) => file,
                Err(_) => {
                    // Assume this error is when there is no EXIF data.
                    return Ok(pic);
                }
            }
        };

        let mut exif = Exif::default();

        exif.description = exif_data
            .get_field(exif::Tag::ImageDescription, exif::In::PRIMARY)
            .map(|e| e.display_value().to_string());

        fn parse_date_time(
            date_time_field: Option<&exif::Field>,
            time_offset_field: Option<&exif::Field>,
        ) -> Option<DateTime<FixedOffset>> {
            let date_time_field = date_time_field?;

            let mut date_time = match date_time_field.value {
                exif::Value::Ascii(ref vec) => exif::DateTime::from_ascii(&vec[0]).ok(),
                _ => None,
            }?;

            if let Some(field) = time_offset_field {
                if let exif::Value::Ascii(ref vec) = field.value {
                    let _ = date_time.parse_offset(&vec[0]);
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

        exif.created_at = parse_date_time(
            exif_data.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY),
            exif_data.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY),
        );
        exif.modified_at = parse_date_time(
            exif_data.get_field(exif::Tag::DateTime, exif::In::PRIMARY),
            exif_data.get_field(exif::Tag::OffsetTime, exif::In::PRIMARY),
        );

        pic.exif = exif.to_option();

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
        let test_data_dir = picture_dir();
        let mut count = 0;
        let s = Scanner::build(&test_data_dir).unwrap();
        s.visit_all(|_| count += 1);
        assert_eq!(5, count);
    }

    #[test]
    fn scan_one() {
        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Sandow.jpg");

        let s = Scanner::build(&test_data_dir).unwrap();
        let pic = s.scan_one(&test_file).unwrap();

        assert!(pic.relative_path.to_str().unwrap().ends_with("Sandow.jpg"));
    }
}
