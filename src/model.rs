use chrono::prelude::*;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PictureInfo {
    // From file system
    pub path: PathBuf,
    pub fs_modified_at: Option<DateTime<Utc>>,

    // From EXIF data
    pub description: Option<String>,
    pub created_at: Option<DateTime<FixedOffset>>,
    pub modified_at: Option<DateTime<FixedOffset>>,
}

impl PictureInfo {
    pub fn new(path: PathBuf) -> PictureInfo {
        PictureInfo {
            path,
            fs_modified_at: None,
            description: None,
            created_at: None,
            modified_at: None,
        }
    }
}
