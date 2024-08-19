// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::gtk;
use strum::EnumString;
use strum::AsRefStr;
use strum::FromRepr;

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

    pub fn scroll_to_end<T: RelmGridItem>(&self, grid: &mut TypedGridView<T, gtk::SingleSelection>) {
        if grid.is_empty() {
            return;
        }

        // We must scroll to a valid index... but we can't get the index of the
        // last item if filters are enabled. So as a workaround disable filters,
        // scroll to end, and then enable filters.

        for i in 0..(grid.filters_len()) {
            grid.set_filter_status(i, false);
        }

        let index = match self {
            AlbumSort::Ascending => grid.len() - 1,
            AlbumSort::Descending => 0,
        };

        grid.view.scroll_to(
            index,
            gtk::ListScrollFlags::SELECT,
            None,
        );

        for i in 0..(grid.filters_len()) {
            grid.set_filter_status(i, true);
        }
    }
}
