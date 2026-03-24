// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::FlatpakPathBuf;
use crate::thumbnailify;

use anyhow::*;
use image::{ImageBuffer, ImageReader, Rgb, RgbImage};
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use tempfile;

use ffmpeg::format::{Pixel, input};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use ffmpeg_next as ffmpeg;

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

        self.thumbnail_internal(path).map_err(|err| {
            let _ = self.thumbnailer.write_failed_thumbnail(path);
            err
        })
    }

    pub fn thumbnail_internal(&self, path: &FlatpakPathBuf) -> Result<()> {
        // Extract first frame of video for thumbnail

        // Temporary output file for frame.
        let temporary_png_file = tempfile::Builder::new().suffix(".png").tempfile()?;

        // See https://docs.rs/ffmpeg-next/latest/src/dump_frames/dump-frames.rs.html
        if let Ok(mut ictx) = input(path.sandbox_path.as_os_str()) {
            let input = ictx
                .streams()
                .best(Type::Video)
                .ok_or(ffmpeg::Error::StreamNotFound)?;

            let video_stream_index = input.index();

            let context_decoder =
                ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
            let mut decoder = context_decoder.decoder().video()?;

            let mut scaler = Context::get(
                decoder.format(),
                decoder.width(),
                decoder.height(),
                Pixel::RGB24,
                decoder.width(),
                decoder.height(),
                Flags::BILINEAR,
            )?;

            let mut frame_index = 0;

            // Lambda for decoding video
            let mut receive_and_process_decoded_frames =
                |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                    let mut decoded = Video::empty();
                    if decoder.receive_frame(&mut decoded).is_ok() {
                        println!("decoded frame");
                        let mut rgb_frame = Video::empty();
                        scaler.run(&decoded, &mut rgb_frame)?;
                        Self::convert_ppm_to_png(&rgb_frame, temporary_png_file.path());
                        frame_index += 1;
                    }
                    Ok(())
                };

            for (stream, packet) in ictx.packets() {
                if stream.index() == video_stream_index {
                    decoder.send_packet(&packet)?;
                    receive_and_process_decoded_frames(&mut decoder)?;
                    break;
                }
            }
            decoder.send_eof()?;
            receive_and_process_decoded_frames(&mut decoder)?;
        }

        let src_image = ImageReader::open(&temporary_png_file)?.decode()?;

        let _ = self.thumbnailer.generate_all_thumbnails(path, src_image)?;

        Ok(())
    }

    /// See https://github.com/lopossumi/Rust-Output-Image/blob/master/src/main.rs
    /// See https://netpbm.sourceforge.net/doc/ppm.html
    fn convert_ppm_to_png(frame: &Video, png_path: &Path) {
        let image_width = frame.width();
        let image_height = frame.height();

        let mut buffer: RgbImage = ImageBuffer::new(image_width, image_height);

        for (x, y, pixel) in buffer.enumerate_pixels_mut() {
            let r = x as f64 / (image_width - 1) as f64;
            let g = y as f64 / (image_height - 1) as f64;
            let b = 0.25;

            let ir = (255.999 * r) as u8;
            let ig = (255.999 * g) as u8;
            let ib = (255.999 * b) as u8;

            *pixel = Rgb([ir, ig, ib]);
        }

        match buffer.save(png_path) {
            Err(e) => eprintln!("Error writing file: {}", e),
            Ok(()) => println!("Done."),
        };
    }
}
