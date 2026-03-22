// SPDX-FileCopyrightText: © 2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use relm4::{Reducer, Reducible};
use std::sync::Arc;

pub type LazyThumbnailState = Arc<Reducer<LazyThumbnailMonitor>>;

#[derive(Debug)]
pub enum LazyThumbnailMonitorInput {
    Completed(VisualId),
}

/// Exposes completed lazy thumbnail loads to subscribers.
pub struct LazyThumbnailMonitor {
    // A thumbnail has been generated for photo or video.
    pub completed: Option<VisualId>,
}

impl Reducible for LazyThumbnailMonitor {
    type Input = LazyThumbnailMonitorInput;

    fn init() -> Self {
        Self { completed: None }
    }

    fn reduce(&mut self, input: Self::Input) -> bool {
        match input {
            LazyThumbnailMonitorInput::Completed(visual_id) => {
                self.completed = Some(visual_id);
                return true; // subscribers only notified if 'true' is returned
            }
        }
    }
}
