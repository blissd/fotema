// SPDX-FileCopyrightText: © 2024-2026 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::*;
use futures::executor::block_on;
use rayon::prelude::*;
use relm4::Reducer;
use relm4::Worker;
use relm4::prelude::*;
use std::path::{Path, PathBuf};
use std::result::Result::Ok;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{error, info};

use std::panic;

use fotema_core::VisualId;
use fotema_core::thumbnailify;
use fotema_core::thumbnailify::ThumbnailSize;
use fotema_core::photo::PhotoThumbnailer;
use fotema_core::video::VideoThumbnailer;


#[derive(Debug)]
pub enum LazyThumbnailTaskInput {
    // Request a thumbnail is generated for a visual item
    Generate(VisualId),

    // Cancel a thumbnail request.
    Cancel(VisualId),

    // Stop all thumbnail generation
    Stop
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
}

impl Worker for LazyThumbnailTask {
    type Init = (
        PathBuf,
        PhotoThumbnailer,
        VideoThumbnailer,
    );
    type Input = LazyThumbnailTaskInput;
    type Output = LazyThumbnailTaskOutput;

    fn init(
        (thumbnails_path, photo_thumbnailer, video_thumbnailer): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        LazyThumbnailTask {
            thumbnails_path: thumbnails_path.into(),
            photo_thumbnailer,
            video_thumbnailer,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {

    }
}
