// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use anyhow::*;
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset};
use exif;
use exif::Exif;
use serde_json::Value;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::process::Command;
use std::result::Result::Ok;

/// Extract EXIF metadata from file
pub fn from_path(path: &Path) -> Result<Metadata> {
    let file = fs::File::open(path)?;
    let file = &mut BufReader::new(file);
    let exif_data = {
        match exif::Reader::new().read_from_container(file) {
            Ok(exif) => exif,
            Err(_) => {
                // Assume this error is when there is no EXIF data.
                return Ok(Metadata::default());
            }
        }
    };

    let mut metadata = from_exif(exif_data)?;
    // FIXME this is too slow. Figure out how to parse maker note in Rust instead of
    // running the exiftool perl script
    // metadata.content_id = content_id(path).ok();
    Ok(metadata)
}

/// Extract EXIF metadata from raw buffer
pub fn from_raw(data: Vec<u8>) -> Result<Metadata> {
    let exif_data = {
        match exif::Reader::new().read_raw(data) {
            Ok(exif) => exif,
            Err(_) => {
                // Assume this error is when there is no EXIF data.
                return Ok(Metadata::default());
            }
        }
    };

    from_exif(exif_data)
}

fn from_exif(exif_data: Exif) -> Result<Metadata> {
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

    let mut metadata = Metadata::default();

    metadata.created_at = parse_date_time(
        exif_data.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY),
        exif_data.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY),
    );
    metadata.modified_at = parse_date_time(
        exif_data.get_field(exif::Tag::DateTime, exif::In::PRIMARY),
        exif_data.get_field(exif::Tag::OffsetTime, exif::In::PRIMARY),
    );

    metadata.lens_model = exif_data
        .get_field(exif::Tag::LensModel, exif::In::PRIMARY)
        .map(|e| e.display_value().to_string());

    Ok(metadata)
}

/// Execute 'exiftool' to extract the content_id from the Apple maker notes.
/// I tried to extract this using the exif-rs library... but just couldn't figure
/// out how to do it.
///
/// FIXME use exif-rs or other Rust code to extract content_id instead of bundling Perl
/// and exiftool in the Flatpak :-(
fn content_id(path: &Path) -> Result<String> {
    let output = Command::new("/app/bin/exiftool")
        .arg("-json")
        .arg("-ContentIdentifier")
        .arg(path.as_os_str())
        .output()?;

    let v: Value = serde_json::from_slice(output.stdout.as_slice())?;

    v[0]["ContentIdentifier"]
        .as_str()
        .map(|x| x.to_string())
        .ok_or_else(|| anyhow!("Missing content id"))
}
