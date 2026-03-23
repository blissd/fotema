// SPDX-FileCopyrightText: © 2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use relm4::{Reducer, Reducible};
use std::sync::Arc;

// Notifies subscribers that thumbnails are ready.
pub type LazyThumbnailNotifier = Arc<Reducer<LazyThumbnailReducible>>;

#[derive(Debug)]
pub enum LazyThumbnailNotifierInput {
    ThumbnailReady(VisualId),
}

pub struct LazyThumbnailReducible {
    // A thumbnail has been generated for photo or video.
    pub visual_id: Option<VisualId>,
}

impl Reducible for LazyThumbnailReducible {
    type Input = LazyThumbnailNotifierInput;

    fn init() -> Self {
        Self { visual_id: None }
    }

    fn reduce(&mut self, input: Self::Input) -> bool {
        match input {
            LazyThumbnailNotifierInput::ThumbnailReady(visual_id) => {
                self.visual_id = Some(visual_id);
                return true; // subscribers only notified if 'true' is returned
            }
        }
    }
}
