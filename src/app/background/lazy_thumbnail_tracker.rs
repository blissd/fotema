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
use tracing::info;

#[derive(Debug)]
struct PendingThumbnail {
    picture: gtk::Picture,
    thumbnail_hash: String,
}

/// Tracks the state of a lazy thumbnail generation request.
#[derive(Debug)]
pub struct LazyThumbnailTracker {
    // Visual waiting for a thumbnail.
    pending: HashMap<VisualId, PendingThumbnail>,

    thumbnailer: Rc<Thumbnailer>,
}

impl LazyThumbnailTracker {
    pub fn new(thumbnailer: Rc<Thumbnailer>) -> Self {
        Self {
            pending: HashMap::new(),
            thumbnailer,
        }
    }

    pub fn add(&mut self, visual: &Visual, picture: gtk::Picture) {
        info!("Adding {:?}", visual.visual_id);
        let pending = PendingThumbnail {
            picture,
            thumbnail_hash: visual.thumbnail_hash(),
        };
        self.pending.insert(visual.visual_id.clone(), pending);
    }

    // A thumbnail has been generated
    pub fn complete(&mut self, visual_id: &VisualId) {
        info!("Completing {:?}", visual_id);
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
        info!("Cancelling {:?}", visual_id);
        self.pending.remove(visual_id);
    }
}
