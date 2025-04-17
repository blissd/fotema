// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::Worker;
use relm4::prelude::*;
use tracing::{error, info};

#[derive(Debug)]
pub enum PhotoScanTaskInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoScanTaskOutput {
    Started,
    Completed,
}

pub struct PhotoScanTask {
    scan: fotema_core::photo::Scanner,
    repo: fotema_core::photo::Repository,
}

impl Worker for PhotoScanTask {
    type Init = (fotema_core::photo::Scanner, fotema_core::photo::Repository);
    type Input = PhotoScanTaskInput;
    type Output = PhotoScanTaskOutput;

    fn init((scan, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { scan, repo }
    }

    fn update(&mut self, msg: PhotoScanTaskInput, sender: ComponentSender<Self>) {
        match msg {
            PhotoScanTaskInput::Start => {
                let result = self.scan_and_add(sender);
                if let Err(e) = result {
                    error!("Failed scan with: {}", e);
                }
            }
        };
    }
}

impl PhotoScanTask {
    fn scan_and_add(&mut self, sender: ComponentSender<Self>) -> std::result::Result<(), String> {
        sender
            .output(PhotoScanTaskOutput::Started)
            .map_err(|e| format!("{:?}", e))?;

        info!("Scanning file system for pictures...");

        let result = self.scan.scan_all().map_err(|e| e.to_string())?;
        info!("Found {} photos to add to database", result.len());

        self.repo.add_all(&result).map_err(|e| e.to_string())?;

        sender
            .output(PhotoScanTaskOutput::Completed)
            .map_err(|e| format!("{:?}", e))
    }
}
