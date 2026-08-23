// SPDX-FileCopyrightText: © 2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use gdk4::prelude::TextureExt;
use image::DynamicImage;

// Download raw RGBA pixels straight from the decoded texture instead of
// round-tripping through a full-resolution PNG encode + decode, which
// dominated photo thumbnailing (the encode alone cost more than the
// actual image decode).
pub fn texture_to_rgba(texture: gdk4::Texture) -> Result<DynamicImage> {
    let width = texture.width() as u32;
    let height = texture.height() as u32;

    let mut downloader = gdk4::TextureDownloader::new(&texture);
    downloader.set_format(gdk4::MemoryFormat::R8g8b8a8);
    let (bytes, stride) = downloader.download_bytes();

    let row_bytes = width as usize * 4;
    let data = if stride == row_bytes {
        bytes.to_vec()
    } else {
        // Strip row padding so the buffer is tightly packed for `image`.
        let mut packed = Vec::with_capacity(row_bytes * height as usize);
        for y in 0..height as usize {
            let start = y * stride;
            packed.extend_from_slice(&bytes[start..start + row_bytes]);
        }
        packed
    };

    let buffer = image::RgbaImage::from_raw(width, height, data)
        .ok_or_else(|| anyhow!("Texture buffer size mismatch"))?;
    Ok(image::DynamicImage::ImageRgba8(buffer))
}
