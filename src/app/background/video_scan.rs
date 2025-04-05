// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::video;
use relm4::Worker;
use relm4::prelude::*;

use tracing::{error, info};

#[derive(Debug)]
pub enum VideoScanInput {
    Start,
}

#[derive(Debug)]
pub enum VideoScanOutput {
    Started,
    Completed,
}

pub struct VideoScan {
    scan: video::Scanner,
    repo: video::Repository,
}

impl Worker for VideoScan {
    type Init = (video::Scanner, video::Repository);
    type Input = VideoScanInput;
    type Output = VideoScanOutput;

    fn init((scan, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { scan, repo }
    }

    fn update(&mut self, msg: VideoScanInput, sender: ComponentSender<Self>) {
        match msg {
            VideoScanInput::Start => {
                let result = self.scan_and_add(sender);
                if let Err(e) = result {
                    error!("Failed scan with: {}", e);
                }
            }
        };
    }
}

impl VideoScan {
    fn scan_and_add(&mut self, sender: ComponentSender<Self>) -> std::result::Result<(), String> {
        sender
            .output(VideoScanOutput::Started)
            .map_err(|e| format!("{:?}", e))?;

        info!("Scanning file system for videos...");

        let result = self.scan.scan_all().map_err(|e| e.to_string())?;
        info!("Found {} videos to add to database", result.len());

        self.repo.add_all(&result).map_err(|e| e.to_string())?;

        sender
            .output(VideoScanOutput::Completed)
            .map_err(|e| format!("{:?}", e))
    }
}
