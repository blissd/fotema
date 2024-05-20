// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::SharedState;

/// The app layout
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Layout {
    // Layout screens for wide devices
    Wide,

    // Layout screens for narrow devices
    Narrow,
}

impl Default for Layout {
    fn default() -> Self {
        Layout::Narrow
    }
}

// Current layout
pub type LayoutState = SharedState<Layout>;
