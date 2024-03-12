use std::path::PathBuf;

#[derive(Debug)]
pub struct PictureInfo {
    pub path: PathBuf,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
