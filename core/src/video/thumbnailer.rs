// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::FlatpakPathBuf;
use crate::thumbnailify;
use crate::thumbnailify::{ThumbnailQuality, ThumbnailSize};
use crate::video::display_matrix::av_display_rotation_get;

use anyhow::Context;
use anyhow::*;
use image::ImageBuffer;
use image::imageops;
use std::result::Result::Ok;
use tracing::debug;

use video_rs::decode::{Decoder, DecoderBuilder};
use video_rs::hwaccel::HardwareAccelerationDeviceType;

use ffmpeg_next::frame::side_data::Type as SideDataType;

/// Thumbnail operations for videos.
#[derive(Debug, Clone)]
pub struct VideoThumbnailer {
    thumbnailer: thumbnailify::Thumbnailer,
}

impl VideoThumbnailer {
    pub fn build(thumbnailer: thumbnailify::Thumbnailer) -> Result<VideoThumbnailer> {
        Ok(VideoThumbnailer { thumbnailer })
    }

    /// Computes a preview for a video
    pub fn thumbnail(&self, path: &FlatpakPathBuf) -> Result<()> {
        if self.thumbnailer.is_failed(&path.host_path) {
            anyhow::bail!("Failed thumbnail marker exists for {:?}", path.host_path);
        }

        self.thumbnail_internal(path, ThumbnailSize::Large, ThumbnailQuality::High)
            .map_err(|err| {
                let _ = self.thumbnailer.write_failed_thumbnail(path);
                err
            })
    }
    /// Computes a preview for a video
    pub fn thumbnail2(
        &self,
        path: &FlatpakPathBuf,
        size: ThumbnailSize,
        quality: ThumbnailQuality,
    ) -> Result<()> {
        if self.thumbnailer.is_failed(&path.host_path) {
            anyhow::bail!("Failed thumbnail marker exists for {:?}", path.host_path);
        }

        self.thumbnail_internal(path, size, quality).map_err(|err| {
            let _ = self.thumbnailer.write_failed_thumbnail(path);
            err
        })
    }

    pub fn thumbnail_internal(
        &self,
        path: &FlatpakPathBuf,
        size: ThumbnailSize,
        quality: ThumbnailQuality,
    ) -> Result<()> {
        // Extract first frame of video for thumbnail

        let mut decoder = Self::build_decoder(path)?;

        let (width, height) = decoder.size();

        // FIXME do we have to decode twice?
        // Right now this is so we can get the image data from frame and the
        // side-data from raw_frame.
        // The image data can also come from raw_frame,
        // but If I use raw_frame.data(0).to_vec() insead of frame.as_slice(),
        // then some frame are corrupted :-/

        let frame = decoder.decode()?.1;
        decoder.seek(0)?;
        let raw_frame = decoder.decode_raw()?;

        let frame_slice = frame
            .as_slice()
            .context("Failed to turn frame into slice.")?;

        let display_matrix = raw_frame.side_data(SideDataType::DisplayMatrix);
        let rotation = if let Some(display_matrix) = display_matrix {
            av_display_rotation_get(display_matrix.data())
        } else {
            f64::NAN
        };

        let buffer: ImageBuffer<image::Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_raw(width, height, frame_slice.to_vec())
                .context("Failed to construct image buffer.")?;

        let buffer = match rotation {
            90.0 => imageops::rotate90(&buffer),
            180.0 | -180.0 => imageops::rotate180(&buffer),
            -90.0 => imageops::rotate270(&buffer),
            _ => buffer,
        };

        let src_image = image::DynamicImage::ImageRgb8(buffer);

        let _ = self
            .thumbnailer
            .generate_thumbnail(path, size, quality, src_image)?;

        Ok(())
    }

    /// Make a decoder. The function prefers VAAPI hardware decode if the
    /// computer has a serviceable VA driver. After each failure the function
    /// uses software decode. Thus this function cannot cause a thumbnail
    /// failure. It can only make the operation faster.
    ///
    /// Set `FOTEMA_DISABLE_VAAPI` to select software decode. Use this variable
    /// if a driver operates incorrectly.
    fn build_decoder(path: &FlatpakPathBuf) -> Result<Decoder> {
        let source = path.sandbox_path.clone();

        if std::env::var_os("FOTEMA_DISABLE_VAAPI").is_some() {
            debug!("VAAPI disabled via FOTEMA_DISABLE_VAAPI; using software decode");
        } else if HardwareAccelerationDeviceType::VaApi.is_available() {
            match DecoderBuilder::new(source.clone())
                .with_hardware_acceleration(HardwareAccelerationDeviceType::VaApi)
                .build()
            {
                Ok(decoder) => {
                    debug!("VAAPI hardware decode enabled for {:?}", path.host_path);
                    return Ok(decoder);
                }
                Err(err) => {
                    debug!(
                        "VAAPI decode unavailable for {:?} ({}); using software decode",
                        path.host_path, err
                    );
                }
            }
        }

        Ok(Decoder::new(source)?)
    }
}
