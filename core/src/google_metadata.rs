// SPDX-FileCopyrightText: © 2025 abb128
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::prelude::*;
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::photo::gps::{GPSCoord, GPSLocation};

#[derive(Debug, Default, Clone)]
struct GoogleMetadata {
    taken_at: Option<DateTime<FixedOffset>>,
    gps_location: Option<GPSLocation>,
}

fn parse_timestamp(ts: &str) -> Option<DateTime<FixedOffset>> {
    let secs = ts.parse::<i64>().ok()?;
    return Some(DateTime::from_timestamp(secs, 0)?.fixed_offset());
}

// I'm not sure if Google truncates filenames based on bytes or codepoints
// Truncating it based on bytes here
fn truncate(s: &mut String, length: usize) {
    while s.len() > length {
        s.pop();
    }
}

// In an ideal world, Google would consistently save metadata to {}.supplemental-metadata.json
// Unfortunately, Google doesn't do this and we get such awesomeness as {}.supplem(1).json, etc
// Reference: https://blog.rpanachi.com/how-to-takeout-from-google-photos-and-fix-metadata-exif-info
fn supplemental_metadata_path(image_path: impl AsRef<Path>) -> Option<PathBuf> {
    let image_path = image_path.as_ref();

    let stem = image_path.file_stem()?.to_string_lossy();
    let ext = image_path.extension()?.to_string_lossy();

    // 1. Simple case, e.g. if user already fixed all names with the script in the linked article
    // e.g. 5142914356_01611c5b98_o.jpg.supplemental-metadata.json
    let candidate = PathBuf::from(
        image_path.with_file_name(format!("{}.{}.supplemental-metadata.json", stem, ext)),
    );
    if candidate.is_file() {
        return Some(candidate);
    }

    // 2. Truncated case. Google seems to truncate the filename before .json to 46 characters
    // e.g. 5142914356_01611c5b98_o.jpg.supplemental-metad.json
    let mut candidate = PathBuf::from(image_path.with_file_name({
        let mut base = format!("{}.{}.supplemental-metadata", stem, ext);
        truncate(&mut base, 46);
        format!("{}.json", base)
    }));
    if candidate.is_file() {
        return Some(candidate);
    }

    // 3. Handle (1), (2), ... suffixes. Google seems to append these to the end of filename before
    // the extension.
    // e.g. 5142914356_01611c5b98_o(1).jpg metadata may actually be
    // 5142914356_01611c5b98_o.jpg.supplemental-metad(1).json
    if let Some(pos) = stem.rfind('(') {
        if stem[pos..].ends_with(')') {
            let clean_stem = &stem[..pos];
            let suffix = &stem[pos..];
            let mut base = format!("{}.{}.supplemental-metadata", clean_stem, ext);
            truncate(&mut base, 46);
            candidate =
                PathBuf::from(image_path.with_file_name(format!("{}{}.json", base, suffix)));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    // 4. Handle -edited suffixes. Google seems to not add any separate metadata for these,
    // so there's supposed to be a non-edited file's metadata to read instead.
    // e.g. 5142914356_01611c5b98_o-edited.jpg metadata is actually
    // 5142914356_01611c5b98_o.jpg.supplemental-metad.json
    // Annoyingly, this is localized based on the user's language. I don't know the translation
    // they use for every language, so I only included the ones I could verify.
    for suffix in &["-redaguota", "-editada", "-edited"] {
        if let Some(pos) = stem.rfind(suffix)
            && stem.ends_with(suffix)
        {
            let clean_stem = &stem[..pos];
            let mut base = format!("{}.{}.supplemental-metadata", clean_stem, ext);
            truncate(&mut base, 46);
            candidate = PathBuf::from(image_path.with_file_name(format!("{}.json", base)));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/*
JSON Structure:
{
  "title": "file-name.jpg",
  "description": "",
  "imageViews": "0",

  // Seems to be the time it was uploaded to Google Photos
  "creationTime": {
    "timestamp": "1688277863",
    "formatted": "2023-07-02 06:04:23 UTC"
  },

  // Actual time it was taken
  "photoTakenTime": {
    "timestamp": "1476983598",
    "formatted": "2016-10-20 17:13:18 UTC"
  },

  "geoData": {
    "latitude": 0.0,
    "longitude": 0.0,
    "altitude": 0.0,
    "latitudeSpan": 0.0,
    "longitudeSpan": 0.0
  },
  "url": "https://photos.google.com/photo/abcd1234efgh5678"
}
*/
fn get_google_metadata(path: &Path) -> Option<GoogleMetadata> {
    let expected_path = supplemental_metadata_path(path)?;
    let mut file = fs::File::open(expected_path).ok()?;

    let mut string = String::new();
    file.read_to_string(&mut string).ok()?;

    let v: Value = serde_json::from_str(&string).ok()?;
    let mut metadata = GoogleMetadata::default();
    let taken_at = &v["photoTakenTime"]["timestamp"];

    if let Value::String(s) = taken_at {
        metadata.taken_at = parse_timestamp(&s);
    }

    let geo_latitude = &v["geoData"]["latitude"];
    let geo_longitude = &v["geoData"]["longitude"];
    if let Value::Number(latitude) = geo_latitude {
        if let Value::Number(longitude) = geo_longitude {
            let lat = latitude.as_f64()?;
            let lon = longitude.as_f64()?;

            if lat != 0.00 && lon != 0.00 {
                let location = GPSLocation {
                    latitude: GPSCoord::decimal_to_gps_coord(lat),
                    longitude: GPSCoord::decimal_to_gps_coord(lon),
                };

                metadata.gps_location = Some(location);
            }
        }
    }
    return Some(metadata);
}

pub fn enrich_photo(
    metadata: Option<crate::photo::Metadata>,
    path: &Path,
) -> Option<crate::photo::Metadata> {
    let mut should_return = metadata.is_some();
    let mut metadata = metadata.unwrap_or(crate::photo::Metadata::default());

    if let Some(v) = get_google_metadata(path) {
        if let Some(location) = v.gps_location {
            metadata.location = Some(location);
            should_return = true;
        }

        if let Some(time) = v.taken_at {
            metadata.created_at = Some(time);
            should_return = true;
        }
    }

    if should_return { Some(metadata) } else { None }
}

pub fn enrich_video(
    metadata: Option<crate::video::Metadata>,
    path: &Path,
) -> Option<crate::video::Metadata> {
    let mut should_return = metadata.is_some();
    let mut metadata = metadata.unwrap_or(crate::video::Metadata::default());

    if let Some(v) = get_google_metadata(path) {
        // TODO: video::Metadata has no location field, but maybe it should
        //if let Some(location) = v.gps_location {
        //    metadata.location = Some(location);
        //    should_return = true;
        //}

        if let Some(time) = v.taken_at {
            metadata.created_at = Some(time.to_utc());
            should_return = true;
        }
    }

    if should_return { Some(metadata) } else { None }
}
