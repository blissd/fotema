// SPDX-FileCopyrightText: © 2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::thumbnailify::{ThumbnailSize, Thumbnailer};
use fotema_core::{Visual, VisualId};
use relm4::gtk;
use relm4::{Reducer, Reducible};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

pub type LazyThumbnailMonitor = Arc<Reducer<LazyThumbnailReducible>>;

#[derive(Debug)]
pub enum LazyThumbnailMonitorInput {
    Completed(VisualId),
}

/// Exposes completed lazy thumbnail loads to subscribers.
pub struct LazyThumbnailReducible {
    // A thumbnail has been generated for photo or video.
    pub completed: Option<VisualId>,
}

impl Reducible for LazyThumbnailReducible {
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

struct PendingThumbnail {
    picture: gtk::Picture,
    thumbnail_hash: String,
}

pub struct PendingThumbnails {
    // Visual waiting for a thumbnail.
    pending: HashMap<VisualId, PendingThumbnail>,

    thumbnailer: Rc<Thumbnailer>,
}

impl PendingThumbnails {
    pub fn add(&mut self, visual: &Visual, picture: gtk::Picture) {
        let pending = PendingThumbnail {
            picture,
            thumbnail_hash: visual.thumbnail_hash(),
        };
        self.pending.insert(visual.visual_id.clone(), pending);
    }

    // A thumbnail has been generated
    pub fn complete(&mut self, visual_id: &VisualId) {
        if let Some(pending) = self.pending.remove(visual_id) {
            // FIXME should respect window width
            let thumbnail_size = ThumbnailSize::Large;
            let thumbnail_path = self
                .thumbnailer
                .nearest_thumbnail(&pending.thumbnail_hash, thumbnail_size);

            if thumbnail_path.is_some() {
                pending.picture.set_filename(thumbnail_path);
                pending.picture.set_content_fit(gtk::ContentFit::Cover);
            }
        }
    }

    pub fn cancel(&mut self, visual_id: &VisualId) {
        self.pending.remove(visual_id);
    }
}
