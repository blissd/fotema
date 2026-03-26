// SPDX-FileCopyrightText: © 2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::app::background::lazy_thumbnail_task::LazyThumbnailTaskInput;
use chrono::*;
use fotema_core::thumbnailify::{ThumbnailSize, Thumbnailer};
use fotema_core::{Visual, VisualId};
use relm4::Sender;
use relm4::gtk;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, trace};

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

    // Send messages to lazy thumbnail task
    sender: relm4::Sender<LazyThumbnailTaskInput>,

    /// Ticker to trigger batch operations
    /// This is to more efficiently send requests to the lazy thumbnail task.
    ticker: crossbeam_channel::Receiver<Instant>,

    /// Items waiting to be sent in a batch.
    /// Many individual message sends cause Fotema to hang.
    add_buffer: Arc<Mutex<HashSet<(VisualId, DateTime<Utc>)>>>,
    cancel_buffer: Arc<Mutex<HashSet<VisualId>>>,

    thumbnailer: Rc<Thumbnailer>,

    // Thumbnails are only generated for the active album view.
    // When a view is deactivated, thumbnail generation should pause.
    // On activation, thumbnail generation resumes.
    is_active: bool,
}

impl LazyThumbnailTracker {
    pub fn new(thumbnailer: Rc<Thumbnailer>, sender: Sender<LazyThumbnailTaskInput>) -> Self {
        let add_buffer = Arc::new(Mutex::new(HashSet::new()));
        let cancel_buffer = Arc::new(Mutex::new(HashSet::new()));
        let ticker = crossbeam_channel::tick(Duration::from_millis(1000));

        {
            let ticker = ticker.clone();
            let add_buffer = add_buffer.clone();
            let cancel_buffer = cancel_buffer.clone();
            let sender = sender.clone();

            thread::spawn(move || {
                loop {
                    let Ok(_tick) = ticker.recv() else {
                        info!("No more ticks");
                        return;
                    };
                    trace!("Tick");

                    let mut add_buffer = add_buffer.lock().unwrap();
                    let mut cancel_buffer = cancel_buffer.lock().unwrap();

                    sender.emit(LazyThumbnailTaskInput::BatchUpdate(
                        add_buffer.clone(),
                        cancel_buffer.clone(),
                    ));

                    (*add_buffer).clear();
                    (*cancel_buffer).clear();
                }
            });
        }

        let tracker = Self {
            pending: HashMap::new(),
            sender,
            thumbnailer,
            is_active: false,
            ticker,
            add_buffer: add_buffer,
            cancel_buffer: cancel_buffer,
        };

        tracker
    }

    pub fn add(&mut self, visual: &Visual, picture: gtk::Picture) {
        if self.pending.contains_key(&visual.visual_id) {
            return;
        }
        //info!("Adding {:?}", visual.visual_id);
        let pending_thumbnail = PendingThumbnail {
            picture,
            thumbnail_hash: visual.thumbnail_hash(),
            ordering_ts: visual.ordering_ts.clone(),
        };
        self.pending
            .insert(visual.visual_id.clone(), pending_thumbnail);

        let mut add_buffer = self.add_buffer.lock().unwrap();
        (*add_buffer).insert((visual.visual_id.clone(), visual.ordering_ts.clone()));

        /*self.sender.emit(LazyThumbnailTaskInput::Generate(
            visual.visual_id.clone(),
            visual.ordering_ts.clone(),
        ));*/
    }

    // A thumbnail has been generated
    pub fn complete(&mut self, visual_id: &VisualId) {
        if let Some(pending_thumbnail) = self.pending.remove(visual_id) {
            info!("Completing {:?}", visual_id);

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
                //info!("Cancelling {:?}", visual_id);

                let mut cancel_buffer = self.cancel_buffer.lock().unwrap();
                (*cancel_buffer).insert(visual_id.clone());
                //self.sender
                //    .emit(LazyThumbnailTaskInput::Cancel(visual_id.clone()));
            }
        }
    }

    pub fn pause(&mut self) {
        if !self.is_active {
            return;
        }
        self.is_active = false;
        info!("Pausing {:?} thumbnails", self.pending.len());

        /* let visual_ids = self
                .pending
                .keys()
                .map(|v| v.clone())
                .collect::<Vec<VisualId>>();

            self.sender.emit(LazyThumbnailTaskInput::Pause(visual_ids));
        */
    }

    pub fn resume(&mut self) {
        if self.is_active {
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
