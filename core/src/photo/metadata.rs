// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use super::gps::GPSLocation;
use super::model::Orientation;
use anyhow::*;
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset};
use exif;
use exif::Exif;
use std::fs;
use std::io::BufReader;
use std::path::Path;
use std::result::Result::Ok;

/// This version number should be incremented each time metadata scanning has
/// a bug fix or feature addition that changes the metadata produced.
/// Each photo will be saved with a metadata scan version which will allow for
/// easy selection of photos when their metadata can be updated.
///
/// History:
/// 0. Initial version.
/// 1. Orientation.
/// 2. Motion photos.
/// 3. GPS coordinates.
pub const VERSION: u32 = 3;

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

    // FIXME what is a better way of doing this?
    //
    // libheif applies the orientation transformation when loading the image,
    // so we must not re-apply the transformation when displaying the image, otherwise
    // we will double transform and show the image incorrectly.
    //
    // To fix that, I'm removing the orientation metadata if the file extension is 'heic'...
    // but it doesn't seem right.
    //
    // Note that this means from_file(...) and from_raw(...) will
    // return inconsistent metadata... again :-(

    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase());

    if ext.is_some_and(|x| x == "heic") {
        metadata.orientation = None;
    }

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
            exif::Value::Ascii(ref vec) if !vec.is_empty() => {
                exif::DateTime::from_ascii(&vec[0]).ok()
            }
            _ => None,
        }?;

        if let Some(field) = time_offset_field {
            match field.value {
                exif::Value::Ascii(ref vec) if !vec.is_empty() => {
                    let _ = date_time.parse_offset(&vec[0]);
                }
                _ => {}
            };
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

    let created_at = parse_date_time(
        exif_data.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY),
        exif_data.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY),
    );

    let modified_at = parse_date_time(
        exif_data.get_field(exif::Tag::DateTime, exif::In::PRIMARY),
        exif_data.get_field(exif::Tag::OffsetTime, exif::In::PRIMARY),
    );

    let lens_model = exif_data
        .get_field(exif::Tag::LensModel, exif::In::PRIMARY)
        .map(|e| e.display_value().to_string());

    // How to orient and flip the image.
    // Note that libheif will automatically apply the transformations when loading the image
    // so must be aware of file format before transforming to avoid a double transformation.
    let orientation = exif_data
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|e| e.value.get_uint(0))
        .map(Orientation::from);

    let content_id = ios_content_id(&exif_data);

    let location = gps_location(&exif_data);

    let metadata = Metadata {
        created_at,
        modified_at,
        lens_model,
        orientation,
        content_id,
        location,
    };

    Ok(metadata)
}

/// Parse GPS latitude and longitude from EXIF data
/// Mostly borrowed from Loupe.
/// See https://gitlab.gnome.org/GNOME/loupe/-/blob/main/src/metadata.rs
fn gps_location(exif: &Exif) -> Option<GPSLocation> {
    if let (Some(latitude), Some(latitude_ref), Some(longitude), Some(longitude_ref)) = (
        exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY),
        exif.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY),
        exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY),
        exif.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY),
    ) {
        if let (
            exif::Value::Rational(latitude),
            exif::Value::Ascii(latitude_ref),
            exif::Value::Rational(longitude),
            exif::Value::Ascii(longitude_ref),
        ) = (
            &latitude.value,
            &latitude_ref.value,
            &longitude.value,
            &longitude_ref.value,
        ) {
            return GPSLocation::for_exif(latitude, latitude_ref, longitude, longitude_ref);
        }
    }

    None
}

/// Parse content ID from the Apple maker note
fn ios_content_id(exif_data: &Exif) -> Option<String> {
    let maker_note = exif_data.get_field(exif::Tag::MakerNote, exif::In::PRIMARY)?;
    let exif::Value::Undefined(ref raw, _offset) = maker_note.value else {
        return None;
    };

    if !raw.starts_with(b"Apple iOS\0") {
        return None;
    }

    // We have an Apple maker note which contains EXIF tags, but they aren't quite in the right format
    // for the exif-rs library to parse.
    //
    // EXIF data starts at byte 12 with a byte order mark, but doesn't include the magic 0x2a.
    //
    // To fix this we rewrite the beginning of the buffer to do the following:
    // 1. Start with a byte order mark (0x4d4d);
    // 2. Have the Douglas constant (0x002a).
    // 3. Have the byte offset point to the first piece of data (14)

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(raw);

    buf[0] = 0x4d; // byte order
    buf[1] = 0x4d; // byte order
    buf[2] = 0;
    buf[3] = 0x2a; // the Douglas constant (42 ;-)
    buf[4] = 0;
    buf[5] = 0;
    buf[6] = 0;
    buf[7] = 14; // first piece of data starts at byte 14

    let exif_data = exif::Reader::new().read_raw(buf).ok()?;

    // 0x11 is the tag ID Apple uses for the content ID.
    let content_id =
        exif_data.get_field(exif::Tag(exif::Context::Tiff, 0x11), exif::In::PRIMARY)?;

    match content_id.value {
        exif::Value::Ascii(ref vecs) if !vecs.is_empty() => {
            let mut bytes = Vec::with_capacity(vecs[0].len());
            bytes.extend_from_slice(&vecs[0]);
            String::from_utf8(bytes).ok()
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_content_id() {
        let dir = env!("CARGO_MANIFEST_DIR");
        let file = Path::new(dir).join("resources/test/Dandelion.jpg");
        let file = fs::File::open(file).unwrap();
        let file = &mut BufReader::new(file);

        let exif_data = exif::Reader::new().read_from_container(file).ok().unwrap();
        let content_id = ios_content_id(&exif_data);

        assert_eq!(
            Some("5D3FF377-55D1-4BFF-A4FF-56B2298FC6C2".to_string()),
            content_id
        );
    }
}
