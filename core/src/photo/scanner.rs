// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Error::*;
use crate::Result;
use chrono;
use chrono::prelude::*;
use exif;
use std::fmt::Display;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub enum ImageFormat {
    Avif,
    Jpeg,
    Png,
    Tiff,
    Webp,
}

impl Display for ImageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = format!("{:?}", self);
        write!(f, "{}", s.to_uppercase())
    }
}

impl ImageFormat {
    fn from(extension: &str) -> Option<ImageFormat> {
        match extension.to_lowercase().as_ref() {
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "webp" => Some(ImageFormat::Webp),
            "avif" => Some(ImageFormat::Avif),
            "tiff" => Some(ImageFormat::Tiff),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ImageSize {
    width: u32,
    height: u32,
}

impl Display for ImageSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} x {}", self.width, self.height)
    }
}

/// A picture on the local file system that has been scanned.
#[derive(Debug, Clone)]
pub struct Picture {
    /// Full path to picture file.
    pub path: PathBuf,

    /// Metadata from the file system.
    pub fs: Option<FsMetadata>,

    /// Metadata from the EXIF tags.
    pub exif: Option<Exif>,

    pub image_format: Option<ImageFormat>,

    pub image_size: Option<ImageSize>,
}

impl Picture {
    /// Creates a new Picture for a given full path.
    pub fn new(path: PathBuf) -> Picture {
        Picture {
            path,
            fs: None,
            exif: None,
            image_format: None,
            image_size: None,
        }
    }

    pub fn created_at(&self) -> Option<chrono::DateTime<Utc>> {
        let exif_ts = self.exif.as_ref().and_then(|x| x.created_at);
        let fs_ts = self.fs.as_ref().and_then(|x| x.created_at);
        exif_ts.map(|x| x.to_utc()).or(fs_ts)
    }

    pub fn modified_at(&self) -> Option<chrono::DateTime<Utc>> {
        let exif_ts = self.exif.as_ref().and_then(|x| x.modified_at);
        let fs_ts = self.fs.as_ref().and_then(|x| x.modified_at);
        exif_ts.map(|x| x.to_utc()).or(fs_ts)
    }
}

/// Metadata from EXIF tags
#[derive(Debug, Default, Clone)]
pub struct Exif {
    pub description: Option<String>,
    pub created_at: Option<DateTime<FixedOffset>>,
    pub modified_at: Option<DateTime<FixedOffset>>,

    /// On iPhone the lens model tells you if it was the front or back camera.
    pub lens_model: Option<String>,
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
#[derive(Debug, Default, Clone)]
pub struct FsMetadata {
    pub created_at: Option<DateTime<Utc>>,
    pub modified_at: Option<DateTime<Utc>>,
    pub file_size_bytes: Option<u64>,
}

impl FsMetadata {
    /// If any fields are present, then wrap self in an Option::Some. Otherwise, return None.
    pub fn to_option(self) -> Option<FsMetadata> {
        if self.created_at.is_some() || self.modified_at.is_some() || self.file_size_bytes.is_some()
        {
            Some(self)
        } else {
            None
        }
    }
}

/// Scans a file system for pictures.
#[derive(Debug, Clone)]
pub struct Scanner {
    /// File system path to scan.
    scan_base: PathBuf,
}

impl Scanner {
    pub fn build(scan_base: &Path) -> Result<Self> {
        fs::create_dir_all(scan_base).map_err(|e| ScannerError(e.to_string()))?;
        let scan_base = PathBuf::from(scan_base);
        Ok(Self { scan_base })
    }

    /// Scans all pictures in the base directory for function `func` to visit.
    pub fn scan_all_visit<F>(&self, func: F)
    where
        F: FnMut(Picture),
    {
        let picture_suffixes = vec![
            String::from("avif"),
            String::from("heic"), // not supported by image-rs
            String::from("jpeg"),
            String::from("jpg"),
            String::from("jxl"),
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

    pub fn scan_all(&self) -> Result<Vec<Picture>> {
        // Count of files in scan_base.
        // Note: no filtering here, so count could be greater than number of pictures.
        // Might want to use the same WalkDir logic in visit_all(...) to get exact count.
        let file_count = WalkDir::new(&self.scan_base).into_iter().count();
        let mut pics = Vec::with_capacity(file_count);
        self.scan_all_visit(|pic| pics.push(pic));
        Ok(pics)
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

        fs.file_size_bytes = file.metadata().ok().map(|x| x.len());

        let mut pic = Picture::new(PathBuf::from(path));
        pic.fs = fs.to_option();

        // TODO don't use extension, get real file type from image bytes
        pic.image_format = ImageFormat::from(
            path.extension()
                .and_then(|x| x.to_str())
                .unwrap_or("unknown"),
        );

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

        let width_opt = exif_data
            .get_field(exif::Tag::ImageWidth, exif::In::PRIMARY)
            .and_then(|x| x.value.get_uint(0));

        let height_opt = exif_data
            .get_field(exif::Tag::ImageLength, exif::In::PRIMARY)
            .and_then(|x| x.value.get_uint(0));

        if let (Some(width), Some(height)) = (width_opt, height_opt) {
            pic.image_size = Some(ImageSize { width, height });
        }

        exif.lens_model = exif_data
            .get_field(exif::Tag::LensModel, exif::In::PRIMARY)
            .map(|e| e.display_value().to_string());

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
    fn scan_all_visit() {
        let test_data_dir = picture_dir();
        let mut count = 0;
        let s = PhotoScanner::build(&test_data_dir).unwrap();
        s.scan_all_visit(|_| count += 1);
        assert_eq!(5, count);
    }

    #[test]
    fn scan_all() {
        let test_data_dir = picture_dir();
        let s = PhotoScanner::build(&test_data_dir).unwrap();
        let mut all = s.scan_all().unwrap();
        assert_eq!(5, all.len());
        all.sort_unstable_by(|a, b| a.path.cmp(&b.path));
        assert!(all[0].path.ends_with("Dog.jpg"));
        assert!(all[1].path.ends_with("Frog.jpg"));
        assert!(all[2].path.ends_with("Kingfisher.jpg"));
        assert!(all[3].path.ends_with("Lavender.jpg"));
        assert!(all[4].path.ends_with("Sandow.jpg"));
    }

    #[test]
    fn scan_one() {
        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Sandow.jpg");

        let s = PhotoScanner::build(&test_data_dir).unwrap();
        let pic = s.scan_one(&test_file).unwrap();

        assert!(pic.path.to_str().unwrap().ends_with("Sandow.jpg"));
    }
}
