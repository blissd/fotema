use chrono::DateTime;
use chrono::FixedOffset;
use chrono::NaiveDateTime;
use std::path::PathBuf;

#[derive(Debug)]
pub struct PictureInfo {
    pub path: PathBuf,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub description: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub modified_at: Option<NaiveDateTime>,
}
