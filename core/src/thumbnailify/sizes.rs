// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

/// Predefined thumbnail sizes conforming to the XDG thumbnail standard.
#[derive(Debug, Clone, Copy)]
pub enum ThumbnailSize {
    Small,
    Normal,
    Large,
    XLarge,
    XXLarge,
}

impl ThumbnailSize {
    /// Converts the thumbnail size into a maximum dimension (in pixels).
    ///
    /// For example:
    /// - Small: 64 pixels
    /// - Normal: 128 pixels
    /// - Large: 256 pixels
    pub fn to_dimension(&self) -> u32 {
        match self {
            ThumbnailSize::Small => 64,
            ThumbnailSize::Normal => 128,
            ThumbnailSize::Large => 256,
            ThumbnailSize::XLarge => 512,
            ThumbnailSize::XXLarge => 1024,
        }
    }
}

impl std::fmt::Display for ThumbnailSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ThumbnailSize::Small => write!(f, "small"),
            ThumbnailSize::Normal => write!(f, "normal"),
            ThumbnailSize::Large => write!(f, "large"),
            ThumbnailSize::XLarge => write!(f, "x-large"),
            ThumbnailSize::XXLarge => write!(f, "xx-large"),
        }
    }
}
