// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::repo;
use crate::Error::*;
use crate::Result;
use image::imageops::FilterType;
use image::io::Reader as ImageReader;
use image::DynamicImage;
use std::path;

const EDGE: u32 = 400;

#[derive(Debug, Clone)]
pub struct Previewer {
    base_path: path::PathBuf,
}

impl Previewer {
    pub fn build(base_path: &path::Path) -> Result<Previewer> {
        let base_path = path::PathBuf::from(base_path);
        std::fs::create_dir_all(base_path.join("square"))
            .map_err(|e| RepositoryError(e.to_string()))?;
        Ok(Previewer { base_path })
    }

    /// Computes a preview square for an image that has been inserted
    /// into the Repository. Preview image will be written to file system.
    pub fn set_preview(&self, pic: &mut repo::Picture) -> Result<()> {
        if pic.square_preview_path.as_ref().is_some_and(|p| p.exists()) {
            return Ok(());
        }

        pic.square_preview_path = None;

        let square = self.from_path(&pic.path)?;

        let square_path = {
            let file_name = format!("{}_{}x{}.jpg", pic.picture_id, EDGE, EDGE);
            self.base_path.join("square").join(file_name)
        };

        // println!("preview = {:?}", square_path);

        square
            .save(&square_path)
            .map_err(|e| PreviewError(format!("image save: {}", e)))?;

        pic.square_preview_path = Some(square_path);

        Ok(())
    }

    /// Computes a preview square for an image on the file system.
    fn from_path(&self, path: &path::Path) -> Result<DynamicImage> {
        let img = ImageReader::open(path)
            .map_err(|e| PreviewError(format!("image open: {}", e)))?
            .decode()
            .map_err(|e| PreviewError(format!("image decode: {}", e)))?;

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

        let target_dir = PathBuf::from("target");
        let prev = Previewer::build(&target_dir).unwrap();
        let img = prev.from_path(&test_file).unwrap();
        let output = target_dir.join("out.jpg");
        let _ = img.save(&output);
    }
}
