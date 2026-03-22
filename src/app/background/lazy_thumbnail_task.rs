// SPDX-FileCopyrightText: © 2024-2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use futures::executor::block_on;
use rayon::prelude::*;
use relm4::Worker;
use relm4::prelude::*;
use relm4::{Reducer, Reducible};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

use std::panic;

use crate::app::SharedState;
use crate::app::background::lazy_thumbnail_monitor::{
    LazyThumbnailMonitor, LazyThumbnailMonitorInput,
};
use fotema_core::VisualId;
use fotema_core::photo::PhotoThumbnailer;
use fotema_core::thumbnailify;
use fotema_core::thumbnailify::ThumbnailSize;
use fotema_core::video::VideoThumbnailer;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

#[derive(Debug)]
pub enum LazyThumbnailTaskInput {
    // Request a thumbnail is generated for a visual item
    Generate(VisualId),

    // Cancel a thumbnail request.
    Cancel(VisualId),

    // Stop all thumbnail generation
    Stop,
}

#[derive(Debug)]
pub enum LazyThumbnailTaskOutput {
    // Thumbnail generation has started.
    Started,

    // Thumbnail generation has completed
    Done(VisualId, PathBuf),
}

pub struct LazyThumbnailTask {
    thumbnails_path: PathBuf,
    photo_thumbnailer: fotema_core::photo::PhotoThumbnailer,
    video_thumbnailer: fotema_core::video::VideoThumbnailer,
    send: mpsc::Sender<VisualId>,
    recv: mpsc::Receiver<VisualId>,

    // Loaded visuals
    shared_state: SharedState,

    // Visuals pending thumbnail generation
    // Map value is count of thumbnail requests.
    pending: Arc<RwLock<HashMap<VisualId, u32>>>,

    lazy_thumbnail_monitor: LazyThumbnailMonitor,
}

impl LazyThumbnailTask {
    pub fn run(&self) {
        let maybe_visual_id: Option<VisualId> = {
            let mut pending = self.pending.read().unwrap();
            pending.keys().nth(0).cloned()
        };
        // get visual
        // generate thumbnail
        // remove from self.pending

        if let Some(visual_id) = maybe_visual_id {
            // remove from self.pending
            {
                let mut pending = self.pending.write().unwrap();
                pending.remove(&visual_id);
            }
            // emit completed event
            self.lazy_thumbnail_monitor
                .emit(LazyThumbnailMonitorInput::Completed(visual_id));
        }
    }
}

impl Worker for LazyThumbnailTask {
    type Init = (
        PathBuf,
        PhotoThumbnailer,
        VideoThumbnailer,
        SharedState,
        LazyThumbnailMonitor,
    );
    type Input = LazyThumbnailTaskInput;
    type Output = LazyThumbnailTaskOutput;

    fn init(
        (
            thumbnails_path,
            photo_thumbnailer,
            video_thumbnailer,
            shared_state,
            lazy_thumbnail_monitor,
        ): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        let (send, recv): (Sender<VisualId>, Receiver<VisualId>) = mpsc::channel();
        LazyThumbnailTask {
            thumbnails_path: thumbnails_path.into(),
            photo_thumbnailer,
            video_thumbnailer,
            send,
            recv,
            shared_state,
            pending: Arc::new(RwLock::new(HashMap::new())),
            lazy_thumbnail_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LazyThumbnailTaskInput::Generate(visual_id) => {}
            LazyThumbnailTaskInput::Cancel(visual_id) => {}
            LazyThumbnailTaskInput::Stop => {}
        };
    }
}
