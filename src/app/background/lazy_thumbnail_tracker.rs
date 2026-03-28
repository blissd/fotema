// SPDX-FileCopyrightText: © 2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::background::lazy_thumbnail_task::LazyThumbnailTaskInput;
use chrono::*;
use fotema_core::thumbnailify::{ThumbnailSize, Thumbnailer};
use fotema_core::{Visual, VisualId};
use relm4::Sender;
use relm4::gtk;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::info;

#[derive(Debug)]
struct PendingThumbnail {
    picture: gtk::Picture,
    thumbnail_hash: String,
    ordering_ts: DateTime<Utc>,
}

/// Tracks the state of a lazy thumbnail generation request.
#[derive(Debug)]
pub struct LazyThumbnailTracker {
    // Visual waiting for a thumbnail.
    pending: HashMap<VisualId, PendingThumbnail>,
    unsent: Arc<Mutex<HashMap<VisualId, DateTime<Utc>>>>,

    // Send messages to lazy thumbnail task
    sender: relm4::Sender<LazyThumbnailTaskInput>,

    /// Ticker to trigger batch operations
    /// This is to more efficiently send requests to the lazy thumbnail task.
    ticker: crossbeam_channel::Receiver<Instant>,

    thumbnailer: Rc<Thumbnailer>,

    // Thumbnails are only generated for the active album view.
    // When a view is deactivated, thumbnail generation should pause.
    // On activation, thumbnail generation resumes.
    is_active: bool,
}

impl LazyThumbnailTracker {
    pub fn new(thumbnailer: Rc<Thumbnailer>, sender: Sender<LazyThumbnailTaskInput>) -> Self {
        let ticker = crossbeam_channel::tick(Duration::from_millis(1000));

        let unsent = Arc::new(Mutex::new(HashMap::<VisualId, DateTime<Utc>>::new()));
        /*
        {
            let ticker = ticker.clone();
            let unsent = unsent.clone();
            let sender = sender.clone();
            thread::spawn(move || {
                loop {
                    let Ok(_tick) = ticker.recv() else {
                        info!("No more ticks");
                        return;
                    };
                    trace!("Tick");

                    let mut unsent = unsent.lock().unwrap();

                    if !unsent.is_empty() {
                        let tuples = (*unsent)
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect::<Vec<(VisualId, DateTime<Utc>)>>();

                        sender.emit(LazyThumbnailTaskInput::Resume(tuples));

                        (*unsent).clear();
                    }
                }
            });
        }*/

        let tracker = Self {
            pending: HashMap::new(),
            unsent: unsent.clone(),
            sender,
            thumbnailer,
            is_active: false,
            ticker,
        };

        tracker
    }

    pub fn add(&mut self, visual: &Visual, picture: gtk::Picture) {
        /* if self.pending.contains_key(&visual.visual_id) {
            return;
        }*/

        //info!("Adding {:?}", visual.visual_id);
        let pending_thumbnail = PendingThumbnail {
            picture,
            thumbnail_hash: visual.thumbnail_hash(),
            ordering_ts: visual.ordering_ts.clone(),
        };

        self.pending
            .insert(visual.visual_id.clone(), pending_thumbnail);

        let tuples = self
            .pending
            .iter()
            .map(|(k, v)| (k.clone(), v.ordering_ts.clone()))
            .collect::<Vec<(VisualId, DateTime<Utc>)>>();

        self.sender.emit(LazyThumbnailTaskInput::Resume(tuples));
    }

    // A thumbnail has been generated
    pub fn complete(&mut self, visual_id: &VisualId) {
        if let Some(pending_thumbnail) = self.pending.remove(visual_id) {
            info!("{} more thumbnails expected.", self.pending.len());

            // FIXME should respect window width
            let thumbnail_size = ThumbnailSize::Large;
            let thumbnail_path = self
                .thumbnailer
                .nearest_thumbnail(&pending_thumbnail.thumbnail_hash, thumbnail_size);

            if thumbnail_path.is_some() {
                pending_thumbnail.picture.set_filename(thumbnail_path);
                pending_thumbnail
                    .picture
                    .set_content_fit(gtk::ContentFit::Cover);
            }
        }
    }

    pub fn cancel(&mut self, visual_id: &VisualId) {
        if self.pending.remove(visual_id).is_some() {
            // if not active, then cancellation previously sent
            if self.is_active {
                self.sender
                    .emit(LazyThumbnailTaskInput::Cancel(visual_id.clone()));
                /*
                let tuples = self
                    .pending
                    .iter()
                    .map(|(k, v)| (k.clone(), v.ordering_ts.clone()))
                    .collect::<Vec<(VisualId, DateTime<Utc>)>>();

                self.sender.emit(LazyThumbnailTaskInput::Resume(tuples));
                */
            }
        }
    }

    pub fn pause(&mut self) {
        if !self.is_active {
            return;
        }
        self.is_active = false;
    }

    pub fn resume(&mut self) {
        if self.is_active || self.pending.is_empty() {
            return;
        }

        info!("Resuming {:?} thumbnails", self.pending.len());
        self.is_active = true;
        let tuples = self
            .pending
            .iter()
            .map(|(k, v)| (k.clone(), v.ordering_ts.clone()))
            .collect::<Vec<(VisualId, DateTime<Utc>)>>();

        self.sender.emit(LazyThumbnailTaskInput::Resume(tuples));
    }

    pub fn clear(&mut self) {
        self.pause();
        self.pending.clear();
    }
}
