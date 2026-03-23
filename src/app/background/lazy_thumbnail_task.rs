// SPDX-FileCopyrightText: © 2024-2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use futures::executor::block_on;
use rayon::prelude::*;
use relm4::Worker;
use relm4::prelude::*;
use std::collections::HashMap;
use std::result::Result::Ok;
use std::sync::{Arc, RwLock};

use std::panic;

use crate::app::SharedState;
use crate::app::background::lazy_thumbnail_notifier::{
    LazyThumbnailNotifier, LazyThumbnailNotifierInput,
};
use fotema_core::Visual;
use fotema_core::VisualId;
use fotema_core::photo::PhotoThumbnailer;
use fotema_core::thumbnailify;
use fotema_core::thumbnailify::ThumbnailSize;
use fotema_core::video::VideoThumbnailer;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use tracing::{error, info};

#[derive(Debug)]
pub enum LazyThumbnailTaskInput {
    // Request a thumbnail is generated for a visual item
    Generate(VisualId),

    // Cancel a thumbnail request.
    Cancel(VisualId),

    // Thumbnail generated.
    Done(VisualId),

    // Stop all thumbnail generation
    Stop,
}

#[derive(Debug)]
pub enum LazyThumbnailTaskOutput {
    // Thumbnail generation has completed
    ThumbnailReady(VisualId),
}

pub struct LazyThumbnailTask {
    runner: Arc<Runner>,

    send: mpsc::Sender<VisualId>,

    // Visuals pending thumbnail generation
    // Map value is count of thumbnail requests.
    pending: Arc<RwLock<HashMap<VisualId, u32>>>,

    lazy_thumbnail_notifier: LazyThumbnailNotifier,
}

impl LazyThumbnailTask {
    fn process_next(&self, pending: &HashMap<VisualId, u32>) {
        if let Some(visual_id) = pending.keys().nth(0).cloned() {
            let _ = self.send.send(visual_id);
        }
    }
}

impl Worker for LazyThumbnailTask {
    type Init = (
        PhotoThumbnailer,
        fotema_core::photo::Repository,
        VideoThumbnailer,
        fotema_core::video::Repository,
        SharedState,
        LazyThumbnailNotifier,
    );
    type Input = LazyThumbnailTaskInput;
    type Output = LazyThumbnailTaskOutput;

    fn init(
        (
            photo_thumbnailer,
            photo_repo,
            video_thumbnailer,
            video_repo,
            shared_state,
            lazy_thumbnail_notifier,
        ): Self::Init,
        sender: ComponentSender<Self>,
    ) -> Self {
        let (send, recv): (Sender<VisualId>, Receiver<VisualId>) = mpsc::channel();

        let runner = Arc::new(Runner {
            sender: sender.input_sender().clone(),
            shared_state,
            visuals: Arc::new(RwLock::new(HashMap::new())),
            photo_thumbnailer,
            photo_repo,
            video_thumbnailer,
            video_repo,
        });

        {
            let runner = runner.clone();
            thread::spawn(move || {
                runner.run(recv);
            });
        }

        LazyThumbnailTask {
            runner: runner,
            send,
            pending: Arc::new(RwLock::new(HashMap::new())),
            lazy_thumbnail_notifier,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LazyThumbnailTaskInput::Generate(visual_id) => {
                let mut pending = self.pending.write().unwrap();
                pending
                    .entry(visual_id.clone())
                    .and_modify(|counter| *counter += 1)
                    .or_insert(1);

                if pending.len() == 1 {
                    self.process_next(&(*pending));
                }
            }
            LazyThumbnailTaskInput::Done(visual_id) => {
                let mut pending = self.pending.write().unwrap();
                pending.remove(&visual_id);
                self.process_next(&(*pending));
                let _ = sender.output(LazyThumbnailTaskOutput::ThumbnailReady(visual_id));
            }
            LazyThumbnailTaskInput::Cancel(visual_id) => {
                let mut pending = self.pending.write().unwrap();
                pending
                    .entry(visual_id.clone())
                    .and_modify(|counter| *counter -= 1);
                if let Some(0) = pending.get(&visual_id) {
                    info!("Cancelled entry");
                    pending.remove(&visual_id);
                }
            }
            LazyThumbnailTaskInput::Stop => {
                let mut pending = self.pending.write().unwrap();
                pending.clear();
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
                }
                if visual.video_path.is_some() && visual.video_id.is_some() {
                    self.generate_video_thumbnail(&visual);
                } else {
                    info!(
                        "Ignoring visual {:?} because no picture or video path.",
                        visual_id
                    );
                }
            }

            let _ = self.sender.send(LazyThumbnailTaskInput::Done(visual_id));
        }
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
