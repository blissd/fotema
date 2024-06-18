// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;
use fotema_core::Visual;
use h3o::CellIndex;
use fotema_core::VisualId;

// An album is a view applied over the whole collection of messages.
// An AlbumFilter defines the filter to apply to produce an album.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlbumFilter {
    // Show no photos
    None,

    // Show a single photo
    One(VisualId),

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

    // Show photos in a geographic area
    GeographicArea(CellIndex),
}

impl AlbumFilter {
    pub fn filter(self, v: &Visual) -> bool {
        match self {
            AlbumFilter::None => false,
            AlbumFilter::One(visual_id) => v.visual_id == visual_id,
            AlbumFilter::All => true,
            AlbumFilter::Folder(path) => v.parent_path == path,
            AlbumFilter::Motion => v.is_motion_photo(),
            AlbumFilter::Selfies => v.is_selfie(),
            AlbumFilter::Videos => v.is_video_only() && !v.is_motion_photo(),
            AlbumFilter::GeographicArea(cell_index) => {
                if let Some(location) = v.location {
                    let cell = location.to_cell(cell_index.resolution());
                    cell == cell_index
                } else {
                    false
                }
            },
        }
    }
}
