// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::Error::*;
use crate::Result;
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use std::path;

/// A square version of a picture with a resolution normalized to 400x400.
pub struct SquarePicture {}

const EDGE: u32 = 400;

/// Create a square copy of an image
pub fn to_square(path: &path::Path) -> Result<DynamicImage> {
    let img = ImageReader::open(path)
        .map_err(|e| PreviewError(e.to_string()))?
        .decode()
        .map_err(|e| PreviewError(e.to_string()))?;

    let img = if img.width() == img.height() && img.width() == EDGE {
        return Ok(img); // the perfect image for previewing :-)
                        //return Ok(img.resize(EDGE, EDGE, FilterType::Nearest));
    } else if img.width() == img.height() {
        img
    } else if img.width() < img.height() {
        let h = (img.height() - img.width()) / 2;
        img.crop_imm(0, h, img.width(), img.width())
    } else {
        let w = (img.width() - img.height()) / 2;
        img.crop_imm(w, 0, img.height(), img.height())
    };

    let img = img.resize(EDGE, EDGE, FilterType::Triangle);
    Ok(img)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn picture_dir() -> PathBuf {
        let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_data_dir.push("resources/test");
        test_data_dir
    }

    #[test]
    fn test_to_square() {
        let test_data_dir = picture_dir();
        let mut test_file = test_data_dir.clone();
        test_file.push("Frog.jpg");

        let img = to_square(&test_file).unwrap();
        let output = path::Path::new("out.jpg");
        let _ = img.save(&output);
    }
}
