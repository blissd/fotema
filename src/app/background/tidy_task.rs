// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::Result;
use relm4::Worker;
use relm4::prelude::*;
use relm4::gtk::glib;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::{error, info};

use crate::APP_ID;

#[derive(Debug)]
pub enum TidyTaskInput {
    Start,
}

#[derive(Debug)]
pub enum TidyTaskOutput {
    Started,
    Completed,
}

pub struct TidyTask {
    // Stop flag
    stop: Arc<AtomicBool>,
}

impl TidyTask {
    fn tidy(&self, sender: &ComponentSender<TidyTask>) -> Result<()> {

        let _= sender.output(TidyTaskOutput::Started);

        // TODO remove me after 2026-01-01
        // Delete legacy thumbnail directory
        let legacy_dir = glib::user_cache_dir()
            .join(APP_ID)
            .join("photo_thumbnails");

        if legacy_dir.exists() {
            std::fs::remove_dir_all(legacy_dir)?;
        }

        // TODO remove me after 2026-01-01
        // Delete legacy thumbnail directory
        let legacy_dir = glib::user_cache_dir()
            .join(APP_ID)
            .join("video_thumbnails");

        if legacy_dir.exists() {
            std::fs::remove_dir_all(legacy_dir)?;
        }

        // TODO remove me after 2026-01-01
        // Delete legacy video transcode directory
        let legacy_dir = glib::user_cache_dir()
            .join(APP_ID)
            .join("video_transcodes");

        if legacy_dir.exists() {
            std::fs::remove_dir_all(legacy_dir)?;
        }

        let _= sender.output(TidyTaskOutput::Completed);

        Ok(())
    }
}

impl Worker for TidyTask {
    type Init = Arc<AtomicBool>;
    type Input = TidyTaskInput;
    type Output = TidyTaskOutput;

    fn init(stop: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { stop }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        if self.stop.load(Ordering::Relaxed) {
            let _= sender.output(TidyTaskOutput::Completed);
            return;
        }

        match msg {
            TidyTaskInput::Start => {
                info!("Tidying up...");

                if let Err(e) = self.tidy(&sender) {
                    error!("Failed to tidy: {}", e);
                }
            }
        };
    }
}
