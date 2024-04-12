// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::video::repo;
use crate::Error::*;
use crate::Result;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use std::path;
use std::process::Command;
use tempfile;

const EDGE: u32 = 200;

#[derive(Debug, Clone)]
pub struct Thumbnailer {
    base_path: path::PathBuf,
}

impl Thumbnailer {
    pub fn build(base_path: &path::Path) -> Result<Self> {
        let base_path = path::PathBuf::from(base_path);
        std::fs::create_dir_all(base_path.join("square"))
            .map_err(|e| RepositoryError(e.to_string()))?;
        Ok(Self { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub fn set_thumbnail(&self, vid: repo::Video) -> Result<repo::Video> {
        if vid.thumbnail_path.as_ref().is_some_and(|p| p.exists()) {
            return Ok(vid);
        }

        let png_file = tempfile::Builder::new()
            .suffix(".png")
            .tempfile()
            .map_err(|e| PreviewError(format!("Temp file: {}", e)))?;

        // ffmpeg is installed as a flatpak extension.
        Command::new("/usr/bin/ffmpeg")
            .arg("-loglevel")
            .arg("error")
            .arg("-y") // temp file will already exist, so allow overwriting
            .arg("-i")
            .arg(vid.path.as_os_str())
            .arg("-update")
            .arg("true")
            .arg("-vf")
            .arg(r"select=eq(n\,0)") // select frame zero
            .arg(png_file.path())
            .status()
            .map_err(|e| PreviewError(format!("ffmpeg result: {}", e)))?;

        let square = self.standard_thumbnail(png_file.path())?;

        let square_path = {
            let file_name = format!("{}_{}x{}.png", vid.video_id, EDGE, EDGE);
            self.base_path.join("square").join(file_name)
        };

        let result = square
            .save(&square_path)
            .map_err(|e| PreviewError(format!("image save: {}", e)));

        let mut vid = vid;

        if result.is_err() {
            let _ = std::fs::remove_file(&square_path);
            result?;
        } else {
            vid.thumbnail_path = Some(square_path);
        }

        Ok(vid)
    }

    // FIXME copy-and-paste from photo thumbnailer
    fn standard_thumbnail(&self, path: &path::Path) -> Result<DynamicImage> {
        let img = ImageReader::open(path)
            .map_err(|e| PreviewError(format!("image open: {}", e)))?
            .decode()
            .map_err(|e| PreviewError(format!("image decode: {}", e)))?;

        let img = if img.width() == img.height() && img.width() == EDGE {
            return Ok(img);
        } else if img.width() == img.height() {
            img
        } else if img.width() < img.height() {
            let h = (img.height() - img.width()) / 2;
            img.crop_imm(0, h, img.width(), img.width())
        } else {
            let w = (img.width() - img.height()) / 2;
            img.crop_imm(w, 0, img.height(), img.height())
        };

        let img = img.thumbnail(EDGE, EDGE);
        Ok(img)
    }
}
