// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use anyhow::*;
use chrono::prelude::*;
use chrono::{DateTime, TimeDelta};

use ffmpeg_next as ffmpeg;
//use ffmpeg_next::frame::side_data::Type as SideDataType;
use crate::video::display_matrix::av_display_rotation_get;
use ffmpeg_next::packet::side_data::Type as SideDataType;

use std::path::Path;
use std::result::Result::Ok;

use std::fs;

/// This version number should be incremented each time metadata scanning has
/// a bug fix or feature addition that changes the metadata produced.
/// Each photo will be saved with a metadata scan version which will allow for
/// easy selection of videos when there metadata can be updated.
//
// 1. ???
// 2. ???

pub const VERSION: u32 = 2;

pub fn from_path(path: &Path) -> Result<Metadata> {
    let mut metadata = Metadata::default();

    let fs_metadata = fs::metadata(path)?;
    metadata.fs_created_at = fs_metadata.created().map(Into::<DateTime<Utc>>::into).ok();
    metadata.fs_modified_at = fs_metadata.modified().map(Into::<DateTime<Utc>>::into).ok();

    let context = ffmpeg::format::input(path)?;

    let context_metadata = context.metadata();

    metadata.stream_created_at = context_metadata.get("creation_time").and_then(|x| {
        let dt = DateTime::parse_from_rfc3339(x).ok();
        dt.map(|y| y.to_utc())
    });

    metadata.content_id = context_metadata
        .get("com.apple.quicktime.content.identifier")
        .map(String::from);

    metadata.container_format = Some(String::from(context.format().description()));

    if let Some(stream) = context.streams().best(ffmpeg::media::Type::Video) {
        let duration = stream.duration() as f64 * f64::from(stream.time_base()) * 1000.0;
        metadata.duration = TimeDelta::try_milliseconds(duration as i64);

        let stream_metadata = stream.metadata();

        metadata.stream_created_at = metadata.stream_created_at.or_else(|| {
            stream_metadata.get("creation_time").and_then(|x| {
                let dt = DateTime::parse_from_rfc3339(x).ok();
                dt.map(|y| y.to_utc())
            })
        });

        let codec = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        metadata.video_codec = Some(String::from(codec.id().name()));

        if let Ok(video) = codec.decoder().video() {
            metadata.width = Some(video.width() as u64);
            metadata.height = Some(video.height() as u64);
        }

        let display_matrix = stream
            .side_data()
            .find(|item| item.kind() == SideDataType::DisplayMatrix);
        let rotation = if let Some(display_matrix) = display_matrix {
            av_display_rotation_get(display_matrix.data())
        } else {
            f64::NAN
        };

        if !f64::is_nan(rotation) {
            metadata.rotation = Some(rotation as i32);
            println!(
                "rotation f64={}, metadata.rotation={:?}",
                rotation, metadata.rotation
            );
        }
    }

    if let Some(stream) = context.streams().best(ffmpeg::media::Type::Audio) {
        let codec = ffmpeg::codec::context::Context::from_parameters(stream.parameters())?;
        metadata.audio_codec = Some(String::from(codec.id().name()));
    }

    Ok(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_next() {
        ffmpeg::init().unwrap();

        let dir = env!("CARGO_MANIFEST_DIR");
        //let file = Path::new(dir).join("/var/home/david/Pictures/Test/raw_heic/IMG_9835.MOV");
        let file = Path::new(dir).join("/var/home/david/Pictures/Test/Compatible/IMG_7354.MOV");
        let metadata = from_path(&file).unwrap();
        println!("metadata = {:?}", metadata);
        //let file = fs::File::open(file).unwrap();
        //let file = &mut BufReader::new(file);
    }
}
