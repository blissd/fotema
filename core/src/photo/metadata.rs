// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use exif;
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset, Utc};

#[derive(Debug)]
pub struct Metadata {

    pub created_at: Option<DateTime<FixedOffset>>,

    pub modified_at: Option<DateTime<FixedOffset>>,

    /// On iPhone the lens model tells you if it was the front or back camera.
    pub lens_model: Option<String>,
}

impl Metadata {
    fn build(path: &Path) -> Result<Metadata> {
        let file = fs::File::open(path).map_err(|e| ScannerError(e.to_string()))?;

        let exif_data = {
            let f = &mut BufReader::new(file);
            match exif::Reader::new().read_from_container(f) {
                Ok(file) => file,
                Err(_) => {
                    // Assume this error is when there is no EXIF data.
                    return Ok(());
                }
            }
        };

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

        extra.exif_created_at = parse_date_time(
            exif_data.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY),
            exif_data.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY),
        );
        extra.exif_modified_at = parse_date_time(
            exif_data.get_field(exif::Tag::DateTime, exif::In::PRIMARY),
            exif_data.get_field(exif::Tag::OffsetTime, exif::In::PRIMARY),
        );

        extra.exif_lens_model = exif_data
            .get_field(exif::Tag::LensModel, exif::In::PRIMARY)
            .map(|e| e.display_value().to_string());

        Ok(())
    }
}
