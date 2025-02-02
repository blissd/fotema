// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

use fotema_core::PictureId;
use fotema_core::Visual;
use fotema_core::VisualId;
use h3o::CellIndex;

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

    /// Show photos who's picture_id is in a set. Used for person filtering.
    /// FIXME should probably be a Set of some kind... but that mucks up PartialEq and Eq.
    Any(Vec<PictureId>),
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
            }
            AlbumFilter::Any(picture_ids) => {
                v.picture_id.is_some_and(|id| picture_ids.contains(&id))
            }
        }
    }
}
