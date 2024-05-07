// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use tracing::{event, Level};

#[derive(Debug)]
pub enum ScanPhotosInput {
    Start,
}

#[derive(Debug)]
pub enum ScanPhotosOutput {
    Started,
    Completed,
}

pub struct ScanPhotos {
    scan: fotema_core::photo::Scanner,
    repo: fotema_core::photo::Repository,
}

impl Worker for ScanPhotos {
    type Init = (fotema_core::photo::Scanner, fotema_core::photo::Repository);
    type Input = ScanPhotosInput;
    type Output = ScanPhotosOutput;

    fn init((scan, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { scan, repo }
    }

    fn update(&mut self, msg: ScanPhotosInput, sender: ComponentSender<Self>) {
        match msg {
            ScanPhotosInput::Start => {
                let result = self.scan_and_add(sender);
                if let Err(e) = result {
                    event!(Level::ERROR, "Failed scan with: {}", e);
                }
            }
        };
    }
}

impl ScanPhotos {
    fn scan_and_add(&mut self, sender: ComponentSender<Self>) -> std::result::Result<(), String> {

        sender.output(ScanPhotosOutput::Started)
            .map_err(|e| format!("{:?}", e))?;

        event!(Level::INFO, "Scanning file system for pictures...");
        let result = self.scan.scan_all().map_err(|e| e.to_string())?;
        self.repo.add_all(&result).map_err(|e| e.to_string())?;

        sender.output(ScanPhotosOutput::Completed)
            .map_err(|e| format!("{:?}", e))

    }
}
