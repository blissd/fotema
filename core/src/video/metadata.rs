// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::Metadata;
use anyhow::*;
use chrono::{DateTime, TimeDelta};
use ffmpeg_next as ffmpeg;
use std::path::Path;
use std::result::Result::Ok;

/// This version number should be incremented each time metadata scanning has
/// a bug fix or feature addition that changes the metadata produced.
/// Each photo will be saved with a metadata scan version which will allow for
/// easy selection of videos when there metadata can be updated.

pub const VERSION: u32 = 1;

pub fn from_path(path: &Path) -> Result<Metadata> {
    let mut metadata = Metadata::default();

    let context = ffmpeg::format::input(path)?;

    let context_metadata = context.metadata();

    metadata.created_at = context_metadata.get("creation_time").and_then(|x| {
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

        metadata.created_at = metadata.created_at.or_else(|| {
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
        let file = Path::new(dir).join("/var/home/david/Pictures/Test/raw_heic/IMG_9835.MOV");
        let _metadata = from_path(&file).unwrap();
        //let file = fs::File::open(file).unwrap();
        //let file = &mut BufReader::new(file);
    }
}
