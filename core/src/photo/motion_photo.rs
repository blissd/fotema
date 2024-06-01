// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::photo::model::PictureId;
use anyhow::*;

use super::model::MotionPhotoVideo;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use tracing::debug;

use sm_motion_photo::SmMotion;

use crate::video::metadata as video_metadata;
use crate::video::transcode;

/// This version number should be incremented each motion photo extraction has
/// a bug fix or feature addition that changes the motion photo data produced.
/// Each photo will be saved with a motion photo extraction version which will allow for
/// easy selection of photos when their motion photo can be updated.

pub const VERSION: u32 = 1;

/// Motion photos are an image followed by an embedded MP4 video.
#[derive(Debug, Clone)]
pub struct MotionPhotoExtractor {
    base_path: PathBuf,
}

impl MotionPhotoExtractor {
    pub fn build(base_path: &Path) -> Result<MotionPhotoExtractor> {
        let base_path = PathBuf::from(base_path).join("motion_photos");
        std::fs::create_dir_all(&base_path)?;

        Ok(MotionPhotoExtractor { base_path })
    }

    /// Extract motion photo video if it exists.
    pub fn extract(
        &self,
        picture_id: &PictureId,
        picture_path: &Path,
    ) -> Result<Option<MotionPhotoVideo>> {
        let photo_file = File::open(picture_path)?;
        let Some(sm) = SmMotion::with(&photo_file) else {
            return Ok(None); // would be nice if API returned a result instead of an option.
        };

        if !sm.has_video() {
            return Ok(None);
        }

        debug!("Photo {:?} has an embedded motion video.", picture_path);

        let video_path = {
            // Create a directory per 1000 motion photos
            let partition = (picture_id.id() / 1000) as i32;
            let partition = format!("{:0>4}", partition);
            let file_name = format!("{}.mp4", picture_id);
            self.base_path.join(partition).join(file_name)
        };

        if !video_path.exists() {
            video_path.parent().map(|p| {
                let _ = std::fs::create_dir_all(p);
            });

            let mut video_file = File::create(&video_path).unwrap();
            sm.dump_video_file(&mut video_file).unwrap();
        }

        let mut mpv = MotionPhotoVideo {
            path: video_path.clone(),
            duration: None,
            video_codec: None,
            rotation: None,
            transcoded_path: None,
        };

        if let Ok(meta) = video_metadata::from_path(&video_path) {
            mpv.video_codec = meta.video_codec;
            mpv.rotation = meta.rotation;
            mpv.duration = meta.duration;
        } else {
            // If we have extracted the video but can't get any metadata, then the motion photo
            // format for this file probably isn't supported by the sm_motion_photo library and
            // we have duff data.
            return Ok(None);
        }

        if mpv
            .video_codec
            .as_ref()
            .is_some_and(|codec| codec == "hevc")
        {
            let transcoded_path = {
                // Create a directory per 1000 motion photos
                let partition = (picture_id.id() / 1000) as i32;
                let partition = format!("{:0>4}", partition);
                let file_name = format!("{}_transcoded.mkv", picture_id);
                self.base_path.join(partition).join(file_name)
            };

            transcode::transcode(&video_path, &transcoded_path)?;

            mpv.transcoded_path = Some(transcoded_path);
        }

        Ok(Some(mpv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_motion_photo() {
        // let dir = env!("CARGO_MANIFEST_DIR");
        //let file = Path::new(dir).join("resources/test/Dandelion.jpg");
        let path = Path::new("/var/home/david/Pictures/Test/Motion Photos/photo.jpg");

        let mp = MotionPhotoExtractor::build(&Path::new(".")).unwrap();
        mp.extract(&PictureId::new(123), &path);
    }
}
