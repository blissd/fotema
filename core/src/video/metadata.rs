// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use anyhow::*;
use chrono::{DateTime, TimeDelta};
use jsonpath_rust::JsonPathQuery;
use serde_json::Value;
use std::path::Path;
use std::process::Command;
use std::result::Result::Ok;

/// This version number should be incremented each time metadata scanning has
/// a bug fix or feature addition that changes the metadata produced.
/// Each photo will be saved with a metadata scan version which will allow for
/// easy selection of videos when there metadata can be updated.

pub const VERSION: u32 = 1;

pub fn from_path(path: &Path) -> Result<Metadata> {
    // ffprobe is part of the ffmpeg-full flatpak extension
    // FIXME can video metadata be extracted with the ffmpeg-next Rust library?
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-i")
        .arg(path.as_os_str())
        .arg("-print_format")
        .arg("json")
        .arg("-show_entries")
        .arg("format=duration,format_long_name:format_tags=com.apple.quicktime.content.identifier,com.apple.quicktime.creationdate:stream_tags=creation_time:stream=codec_name,codec_type,width,height")
        .output()?;

    let v: Value = serde_json::from_slice(output.stdout.as_slice())?;

    let mut metadata = Metadata::default();
    metadata.scan_version = VERSION;

    metadata.duration = v["format"]["duration"] // seconds with decimal
        .as_str()
        .and_then(|x| {
            let fractional_secs = x.parse::<f64>();
            let millis = fractional_secs.map(|s| s * 1000.0).ok();
            millis.and_then(|m| TimeDelta::try_milliseconds(m as i64))
        });

    metadata.created_at = v["format"]["tags"]["com.apple.quicktime.creationdate"]
        .as_str()
        .and_then(|x| {
            let dt = DateTime::parse_from_rfc3339(x).ok();
            dt.map(|y| y.to_utc())
        });

    metadata.content_id = v["format"]["tags"]["com.apple.quicktime.content.identifier"]
        .as_str()
        .map(|x| x.to_string());

    metadata.container_format = v["format"]["format_long_name"]
        .as_str()
        .map(|x| x.to_string());

    if let Ok(video_stream) = v.clone().path("$.streams[?(@.codec_type == 'video')]") {
        metadata.video_codec = video_stream[0]["codec_name"]
            .as_str()
            .map(|x| x.to_string());
        metadata.width = video_stream[0]["width"].as_u64();
        metadata.height = video_stream[0]["height"].as_u64();

        let created_at = video_stream[0]["tags"]["creation_time"]
            .as_str()
            .and_then(|x| {
                let dt = DateTime::parse_from_rfc3339(x).ok();
                dt.map(|y| y.to_utc())
            });

        metadata.created_at = metadata.created_at.or_else(|| created_at);
    }

    if let Ok(audio_stream) = v.path("$.streams[?(@.codec_type == 'audio')]") {
        metadata.audio_codec = audio_stream[0]["codec_name"]
            .as_str()
            .map(|x| x.to_string());
    }

    Ok(metadata)
}
