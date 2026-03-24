// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::gtk;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use strum::AsRefStr;
use strum::EnumString;
use strum::FromRepr;
use tracing::info;

// Sort album
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, EnumString, AsRefStr, FromRepr)]
#[repr(u32)]
pub enum AlbumSort {
    // Sort dimension from smallest to largest
    #[default]
    Ascending,

    // Sort dimension from largest to smallest
    Descending,
}

impl AlbumSort {
    pub fn sort<T>(&self, data: &mut [T]) {
        if *self == AlbumSort::Descending {
            data.reverse();
        }
    }

    pub fn scroll_to_end<T: RelmGridItem>(
        &self,
        grid: &mut TypedGridView<T, gtk::SingleSelection>,
    ) {
        if grid.is_empty() {
            return;
        }

        let index = match self {
            AlbumSort::Ascending => grid.len() - 1,
            AlbumSort::Descending => 0,
        };

        // WARN changing the scroll flags to something other than NONE.
        // makes this scroll (usually) be inoperable.
        grid.view.scroll_to(index, gtk::ListScrollFlags::NONE, None);
    }
}
