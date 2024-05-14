// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use fotema_core::Visual;

// An album is a view applied over the whole collection of messages.
// An AlbumFilter defines the filter to apply to produce an album.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlbumFilter {
    // Show no photos
    None,

    // Show all photos
    All,

    // Show only selfies
    Selfies,

    // Show only videos
    Videos,

    // Show only motion photos (live photos)
    Motion,

    // Show photos only for folder
    Folder(PathBuf),
}

impl AlbumFilter {
    pub fn filter(self, v: &Visual) -> bool {
        match self {
            AlbumFilter::None => false,
            AlbumFilter::All => true,
            AlbumFilter::Folder(path) => v.parent_path == path,
            AlbumFilter::Motion => v.is_motion_photo(),
            AlbumFilter::Selfies => v.is_selfie(),
            AlbumFilter::Videos => v.is_video_only() && !v.is_motion_photo(),
        }
    /*
        match self {
            AlbumFilter::None => Box::new(|_| false),
            AlbumFilter::All => Box::new(|_| true),
            AlbumFilter::Folder(path) => Box::new(move |v| v.parent_path == path),
            AlbumFilter::Motion => Box::new(|v| v.is_motion_photo()),
            AlbumFilter::Selfies => Box::new(|v| v.is_selfie()),
            AlbumFilter::Videos => Box::new(|v| v.is_video_only() && !v.is_motion_photo()),
        }
        */
    }
}
