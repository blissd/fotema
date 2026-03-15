// SPDX-FileCopyrightText: © 2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::background::lazy_thumbnail_task::ThumbnailOutcome;
use relm4::{Reducer, Reducible};
use std::sync::Arc;

// Notifies subscribers that thumbnails are ready.
pub type LazyThumbnailNotifier = Arc<Reducer<LazyThumbnailReducible>>;

#[derive(Debug)]
pub enum LazyThumbnailNotifierInput {
    ThumbnailReady(ThumbnailOutcome),
}

pub struct LazyThumbnailReducible {
    // A thumbnail has been generated for photo or video.
    pub outcome: Option<ThumbnailOutcome>,
}

impl Reducible for LazyThumbnailReducible {
    type Input = LazyThumbnailNotifierInput;

    fn init() -> Self {
        Self { outcome: None }
    }

    fn reduce(&mut self, input: Self::Input) -> bool {
        match input {
            LazyThumbnailNotifierInput::ThumbnailReady(outcome) => {
                self.outcome = Some(outcome);
                return true; // subscribers only notified if 'true' is returned
            }
        }
    }
}
