// SPDX-FileCopyrightText: © 2024-2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::executor::block_on;
use relm4::Worker;
use relm4::gtk::glib;
use relm4::prelude::*;
use std::cmp::{Ord, Ordering};
use std::collections::BinaryHeap;
use std::collections::{HashMap, HashSet};
use std::panic;
use std::result::Result::Ok;
use std::sync::{Arc, Mutex, RwLock};

use chrono::*;

use crate::app::SharedState;
use crate::config::APP_ID;
use fotema_core::FlatpakPathBuf;
use fotema_core::Visual;
use fotema_core::VisualId;
use fotema_core::database;
use fotema_core::photo::PhotoThumbnailer;
use fotema_core::video::VideoThumbnailer;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use priority_queue::PriorityQueue;

use std::thread;
use tracing::{error, info, trace};

#[derive(Debug)]
pub enum LazyThumbnailTaskInput {
    // Configure library base directory.
    Configure(FlatpakPathBuf),

    // Refresh visuals
    Refresh,

    // Request a thumbnail is generated for a visual item
    Generate(VisualId, DateTime<Utc>),

    // Cancel a thumbnail request.
    Cancel(VisualId),

    // Thumbnail generated.
    Done(VisualId),

    /// Batch cancel
    Pause(Vec<VisualId>),

    /// Batch add
    Resume(Vec<(VisualId, DateTime<Utc>)>),

    ///
    BatchUpdate(HashMap<VisualId, DateTime<Utc>>, HashSet<VisualId>),

    // Stop all thumbnail generation
    Stop,
}

#[derive(Debug)]
pub enum LazyThumbnailTaskOutput {
    // Thumbnail generation has completed
    ThumbnailReady(VisualId),
}

#[derive(PartialEq, Eq)]
struct OrderedVisualId {
    visual_id: VisualId,
    ordering_ts: DateTime<Utc>,
}

impl Ord for OrderedVisualId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ordering_ts.cmp(&other.ordering_ts)
    }
}

impl PartialOrd for OrderedVisualId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct LazyThumbnailTask {
    con: Arc<Mutex<database::Connection>>,

    shared_state: SharedState,

    runner: Option<Arc<Runner>>,

    send: Sender<VisualId>,

    // Visuals pending thumbnail generation
    pending_ordered: PriorityQueue<VisualId, DateTime<Utc>>,

    photo_thumbnailer: PhotoThumbnailer,
    video_thumbnailer: VideoThumbnailer,
    parallelism: usize,
    inflight: u32,
}

impl LazyThumbnailTask {
    fn process_next(&mut self) {
        if self.runner.is_none() {
            return;
        }

        let count = self.parallelism - self.send.len();
        let mut remaining = count;

        while let Some((visual_id, _)) = self.pending_ordered.pop()
            && remaining > 0
        {
            self.inflight += 1;
            remaining -= 1;
            let _ = self.send.send(visual_id);
        }

        info!(
            "{} inflight. submitted {}. {} remaining.",
            self.inflight,
            count - remaining,
            self.pending_ordered.len()
        );
    }

    fn add(&mut self, visual_id: VisualId, ordering_ts: DateTime<Utc>) {
        self.pending_ordered.push(visual_id, ordering_ts);
    }

    fn cancel(&mut self, visual_id: VisualId) {
        self.pending_ordered.remove(&visual_id);
    }
}

impl Worker for LazyThumbnailTask {
    type Init = (
        Arc<Mutex<database::Connection>>,
        PhotoThumbnailer,
        VideoThumbnailer,
        SharedState,
    );
    type Input = LazyThumbnailTaskInput;
    type Output = LazyThumbnailTaskOutput;

    fn init(
        (con, photo_thumbnailer, video_thumbnailer, shared_state): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        // Unused. Will be replaced when library base directory configured.
        let (send, _recv): (Sender<VisualId>, Receiver<VisualId>) = unbounded();

        let parallelism: usize = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        info!("Available parallelism: {:?}", parallelism);

        let parallelism = usize::max(1, parallelism / 2);
        info!("Lazy thumbnail parallelism: {:?}", parallelism);

        LazyThumbnailTask {
            con,
            shared_state,
            send,
            pending_ordered: PriorityQueue::new(),
            photo_thumbnailer,
            video_thumbnailer,
            runner: None,
            parallelism,
            inflight: 0,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LazyThumbnailTaskInput::Configure(library_base_dir) => {
                info!("Configuring library base directory: {:?}", library_base_dir);
                self.pending_ordered.clear();

                let (send, recv): (Sender<VisualId>, Receiver<VisualId>) =
                    bounded(self.parallelism);
                self.send = send; // should drop and hang-up on previous send channel.

                // TODO this is in many locations
                let data_dir = glib::user_data_dir().join(APP_ID);
                let _ = std::fs::create_dir_all(&data_dir);

                let cache_dir = glib::user_cache_dir().join(APP_ID);
                let _ = std::fs::create_dir_all(&cache_dir);

                let photo_repo = fotema_core::photo::Repository::open(
                    &library_base_dir,
                    &cache_dir,
                    &data_dir,
                    self.con.clone(),
                )
                .unwrap();

                let video_repo = fotema_core::video::Repository::open(
                    &library_base_dir,
                    &cache_dir,
                    &data_dir,
                    self.con.clone(),
                )
                .unwrap();

                let runner = Arc::new(Runner {
                    sender: sender.input_sender().clone(),
                    shared_state: self.shared_state.clone(),
                    visuals: Arc::new(RwLock::new(HashMap::new())),
                    photo_thumbnailer: self.photo_thumbnailer.clone(),
                    photo_repo,
                    video_thumbnailer: self.video_thumbnailer.clone(),
                    video_repo,
                });

                for _ in 1..self.parallelism {
                    let runner = runner.clone();
                    let recv = recv.clone();
                    thread::spawn(move || {
                        runner.run(recv);
                    });
                }

                self.runner = Some(runner);
            }
            LazyThumbnailTaskInput::Generate(visual_id, ordering_ts) => {
                info!("Generate lazy thumbnail request: {:?}", visual_id);
                self.add(visual_id, ordering_ts);
                self.process_next();
            }
            LazyThumbnailTaskInput::Done(visual_id) => {
                trace!("Thumbnails remaining: {}", self.pending_ordered.len());
                if self.inflight > 0 {
                    self.inflight -= 1;
                }
                self.pending_ordered.remove(&visual_id);
                let _ = sender.output(LazyThumbnailTaskOutput::ThumbnailReady(visual_id));
                self.process_next();
            }
            LazyThumbnailTaskInput::Cancel(visual_id) => {
                info!("Cancelled lazy thumbnail: {:?}", visual_id);
                self.cancel(visual_id);
            }
            LazyThumbnailTaskInput::Pause(visual_ids) => {
                info!("Pausing {} lazy thumbnails", visual_ids.len());
                for visual_id in visual_ids {
                    self.cancel(visual_id);
                }
            }
            LazyThumbnailTaskInput::Resume(visual_ids_and_ordering_ts) => {
                info!(
                    "Resuming {} lazy thumbnails",
                    visual_ids_and_ordering_ts.len()
                );
                self.pending_ordered.clear();
                for (visual_id, ordering_ts) in visual_ids_and_ordering_ts {
                    self.add(visual_id, ordering_ts);
                }
                self.process_next();
            }
            LazyThumbnailTaskInput::Stop => {
                self.pending_ordered.clear();
            }
            LazyThumbnailTaskInput::Refresh => {
                if let Some(ref runner) = self.runner {
                    runner.refresh();
                }
            }
            LazyThumbnailTaskInput::BatchUpdate(add, cancel) => {
                if add.is_empty() && cancel.is_empty() {
                    return;
                }

                info!("BatchUpdate(add={}, cancel={})", add.len(), cancel.len());

                for visual_id in &cancel {
                    self.cancel(visual_id.clone());
                }

                for (visual_id, ordering_ts) in add {
                    self.add(visual_id, ordering_ts);
                }
                self.process_next();
            }
        };
    }
}

// Thumbnail generator.
// Receives message.
// Generates thumbnail.
// Sends response.
struct Runner {
    // Send response back to worker task.
    sender: relm4::Sender<LazyThumbnailTaskInput>,

    // Loaded visuals
    shared_state: SharedState,
    visuals: Arc<RwLock<HashMap<VisualId, Arc<Visual>>>>,

    photo_thumbnailer: fotema_core::photo::PhotoThumbnailer,
    photo_repo: fotema_core::photo::Repository,

    video_thumbnailer: fotema_core::video::VideoThumbnailer,
    video_repo: fotema_core::video::Repository,
}

impl Runner {
    // Run forever generating thumbnails.
    pub fn run(&self, recv: Receiver<VisualId>) {
        while let Ok(visual_id) = recv.recv() {
            // get visual
            let maybe_visual: Option<Arc<Visual>> = {
                let visuals = self.visuals.read().unwrap();
                visuals.get(&visual_id).cloned()
            };

            // generate thumbnail
            if let Some(visual) = maybe_visual {
                if visual.picture_path.is_some() && visual.picture_id.is_some() {
                    self.generate_photo_thumbnail(&visual);
                } else if visual.video_path.is_some() && visual.video_id.is_some() {
                    self.generate_video_thumbnail(&visual);
                } else {
                    error!(
                        "Ignoring visual {:?} because no picture or video path. {:?}",
                        visual_id, visual
                    );
                }
            }

            let _ = self.sender.send(LazyThumbnailTaskInput::Done(visual_id));
        }
        info!("Lazy thumbnail runner stopping.");
    }

    // FIXME this is a copy-and-paste from photo_thumbnail_task.rs
    fn generate_photo_thumbnail(&self, visual: &Arc<Visual>) {
        let Some(ref path) = visual.picture_path else {
            todo!()
        };
        let Some(ref picture_id) = visual.picture_id else {
            todo!()
        };

        // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
        // an error but doesn't panic.
        let result = panic::catch_unwind(|| {
            block_on(async { self.photo_thumbnailer.thumbnail(&path).await })
        });

        // If we got an err, then there was a panic.
        // If we got Ok(Err(e)) there wasn't a panic, but we still failed.
        if let Ok(Err(e)) = result {
            error!(
                "Failed generate or add thumbnail: {:?}: Photo path: {:?}",
                e.root_cause(),
                path
            );
            let _ = self.photo_repo.clone().mark_broken(&picture_id);
        } else if result.is_err() {
            error!("Panicked generate or add thumbnail: Photo path: {:?}", path);
            let _ = self.photo_repo.clone().mark_broken(&picture_id);
        }
    }

    // FIXME this is a copy-and-paste from video_thumbnail_task.rs
    fn generate_video_thumbnail(&self, visual: &Arc<Visual>) {
        let Some(ref path) = visual.video_path else {
            todo!()
        };
        let Some(ref video_id) = visual.video_id else {
            todo!()
        };

        // Careful! panic::catch_unwind returns Ok(Err) if the evaluated expression returns
        // an error but doesn't panic.
        let result = panic::catch_unwind(|| self.video_thumbnailer.thumbnail(&path));

        // If we got an err, then there was a panic.
        // If we got Ok(Err(e)) there wasn't a panic, but we still failed.
        if let Ok(Err(e)) = result {
            error!(
                "Failed generate or add thumbnail: {:?}: Video path: {:?}",
                e.root_cause(),
                path
            );
            let _ = self.video_repo.clone().mark_broken(&video_id);
        } else if result.is_err() {
            error!("Panicked generate or add thumbnail: Video path: {:?}", path);
            let _ = self.video_repo.clone().mark_broken(&video_id);
        }
    }

    // Visuals shared state has been updated so rebuild map of VisualId -> Visual.
    fn refresh(&self) {
        let data = self.shared_state.read();

        let mut visuals = self.visuals.write().unwrap();
        visuals.clear();
        data.iter().for_each(|v| {
            visuals.insert(v.visual_id.clone(), v.clone());
        });
    }
}
