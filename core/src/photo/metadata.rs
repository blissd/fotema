// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use anyhow::*;
use byteorder::{BigEndian, ReadBytesExt};
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset};
use exif;
use exif::Exif;
use std::fs;
use std::io::BufReader;
use std::io::Cursor;
use std::path::Path;
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

    let metadata = from_exif(exif_data)?;
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

    metadata.content_id = ios_content_id(&exif_data);

    Ok(metadata)
}

#[derive(Debug)]
enum TagDataType {
    Byte,
    Ascii,
    Short,
    Long,
    Rational,
    SByte,
    Undefined,
    SShort,
    SLong,
    SRational,
    Float,
    Double,
}

impl TagDataType {
    fn from_type_id(id: u16) -> Option<TagDataType> {
        match id {
            1 => Some(TagDataType::Byte),
            2 => Some(TagDataType::Ascii),
            3 => Some(TagDataType::Short),
            4 => Some(TagDataType::Long),
            5 => Some(TagDataType::Rational),
            6 => Some(TagDataType::SByte),
            7 => Some(TagDataType::Undefined),
            8 => Some(TagDataType::SShort),
            9 => Some(TagDataType::SLong),
            10 => Some(TagDataType::SRational),
            11 => Some(TagDataType::Float),
            12 => Some(TagDataType::Double),
            _ => None,
        }
    }

    fn size(&self) -> usize {
        match *self {
            TagDataType::Byte => 1,
            TagDataType::Ascii => 1,
            TagDataType::Short => 2,
            TagDataType::Long => 4,
            TagDataType::Rational => 8,
            TagDataType::SByte => 1,
            TagDataType::Undefined => 1,
            TagDataType::SShort => 2,
            TagDataType::SLong => 4,
            TagDataType::SRational => 8,
            TagDataType::Float => 4,
            TagDataType::Double => 8,
        }
    }
}

/// I've tried to get exif-rs to parse the Apple maker note, but I just can't get
/// it to work. This function is a poor-man's EXIF parser that walks the tags
/// until the content identifier is found and returns it.
/// See https://www.media.mit.edu/pia/Research/deepview/exif.html
fn ios_content_id(exif_data: &Exif) -> Option<String> {
    let maker_note = exif_data.get_field(exif::Tag::MakerNote, exif::In::PRIMARY)?;
    let exif::Value::Undefined(ref raw, _offset) = maker_note.value else {
        return None;
    };

    if !raw.starts_with(b"Apple iOS\0") {
        return None;
    }

    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(raw);
    let mut buf = Cursor::new(buf);
    buf.set_position(14);

    let tag_count = buf.read_u16::<BigEndian>().unwrap();
    //println!("tag count = {}", tag_count);

    loop {
        let tag_id = buf.read_u16::<BigEndian>().ok()?;
        let tag_type_id = buf.read_u16::<BigEndian>().ok()?;
        let tag_type = TagDataType::from_type_id(tag_type_id)?;
        let value_count = buf.read_u32::<BigEndian>().ok()?;

        //println!( "tag_id: {}, tag_type: {:?}, value_count: {}", tag_id, tag_type, value_count );

        if value_count == 0 || value_count > 2024 {
            // Ignore anything that is too big or zero length
            return None;
        }

        let value_len = value_count as usize * tag_type.size();
        //println!("value_len : {}", value_len);

        // If value length is <= 4 bytes, then the next four bytes contain the value.
        // If value length is > 4 bytes, then the next four bytes are an index to where the value is stored.
        // We only care about the content identifier, so just skip the value offset for other fields.
        // Content identifier has an id of 0x11
        if tag_id != 0x11 || value_len <= 4 {
            buf.set_position(buf.position() + 4);
            continue;
        }

        // If we are here, then we have found the content identifier.
        // content identifiers are longer than 4 bytes so we know that we must use the value offset
        let value_offset = buf.read_u32::<BigEndian>().ok()? as usize;
        //println!("value_offset: {}", value_offset);

        let mut value = Vec::with_capacity(value_len);
        let end_idx = value_offset + value_len;
        value.extend_from_slice(&buf.get_ref()[value_offset..end_idx - 1]);
        //println!("raw value:\n{}", pretty_hex(&value));

        let content_id = String::from_utf8(value).ok()?;
        //println!("content_id: {}", content_id);
        return Some(content_id);
    }
    //let id = parse_apple_maker_note_content_id(buf);

    //println!("id = {:?}", id);
    //None
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
            Some("12A813C0-2516-4A6A-BF48-CE453071F714".to_string()),
            content_id
        );
    }
}
