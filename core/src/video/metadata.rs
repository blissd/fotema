// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::prelude::*;
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use jsonpath_rust::{JsonPathFinder, JsonPathInst, JsonPathQuery, JsonPathValue};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

// TODO video::Enricher should use this class for ffprobe parsing

#[derive(Debug, Default, Clone)]
pub struct Metadata {
    pub created_at: Option<DateTime<Utc>>,

    pub width: Option<u32>,

    pub height: Option<u32>,

    pub duration: Option<TimeDelta>,

    pub container_format: Option<String>,

    pub video_codec: Option<String>,

    pub audio_codec: Option<String>,
}

pub enum Error {
    Probe(String),
    Json(String),
}

impl Metadata {
    pub fn from(path: &Path) -> Result<Metadata, Error> {
        // ffprobe is part of the ffmpeg-full flatpak extension
        let output = Command::new("/usr/bin/ffprobe")
            .arg("-v")
            .arg("quiet")
            .arg("-i")
            .arg(path.as_os_str())
            .arg("-print_format")
            .arg("json")
            .arg("-show_entries")
            .arg("format=duration,format_long_name:stream_tags=creation_time:stream=codec_name,codec_type,width,height")
            .output()
            .map_err(|e| Error::Probe(e.to_string()))?;

        let v: Value = serde_json::from_slice(output.stdout.as_slice())
            .map_err(|e| Error::Json(e.to_string()))?;

        let video_stream = v.clone().path("$.streams[@.codec_type = 'video'");
        println!("{:?}", video_stream);

        let created_at = v["streams"][0]["tags"]["creation_time"].as_str();
        let created_at = created_at.and_then(|x| {
            let dt = DateTime::parse_from_rfc3339(x).ok();
            dt.map(|y| y.to_utc())
        });

        let duration = v["format"]["duration"].as_str(); // seconds with decimal
        let duration = duration.and_then(|x| {
            let fractional_secs = x.parse::<f64>();
            let millis = fractional_secs.map(|s| s * 1000.0).ok();
            millis.and_then(|m| TimeDelta::try_milliseconds(m as i64))
        });

        let container_format = v["format"]["format_long_name"].as_str();

        let mut metadata = Metadata {
            created_at,
            width: None,
            height: None,
            duration,
            container_format: container_format.map(|x| x.to_string()),
            video_codec: None,
            audio_codec: None,
        };

        Ok(metadata)
    }
}
