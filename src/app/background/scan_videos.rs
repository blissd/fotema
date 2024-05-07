// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use fotema_core::video;

use tracing::{event, Level};

#[derive(Debug)]
pub enum ScanVideosInput {
    Start,
}

#[derive(Debug)]
pub enum ScanVideosOutput {
    Started,
    Completed,
}

pub struct ScanVideos {
    scan: video::Scanner,
    repo: video::Repository,
}

impl Worker for ScanVideos {
    type Init = (video::Scanner, video::Repository);
    type Input = ScanVideosInput;
    type Output = ScanVideosOutput;

    fn init((scan, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { scan, repo }
    }

    fn update(&mut self, msg: ScanVideosInput, sender: ComponentSender<Self>) {
        match msg {
            ScanVideosInput::Start => {
                let result = self.scan_and_add(sender);
                if let Err(e) = result {
                    event!(Level::ERROR, "Failed scan with: {}", e);
                }
            }
        };
    }
}

impl ScanVideos {
    fn scan_and_add(&mut self, sender: ComponentSender<Self>) -> std::result::Result<(), String> {

        sender.output(ScanVideosOutput::Started)
            .map_err(|e| format!("{:?}", e))?;

        event!(Level::INFO, "Scanning file system for videos...");
        let result = self.scan.scan_all().map_err(|e| e.to_string())?;
        self.repo.add_all(&result).map_err(|e| e.to_string())?;

        sender.output(ScanVideosOutput::Completed)
            .map_err(|e| format!("{:?}", e))

    }
}
