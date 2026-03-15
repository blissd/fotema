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


