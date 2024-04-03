// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum ScanPhotosInput {
    ScanAll,
}

#[derive(Debug)]
pub enum ScanPhotosOutput {
    ScanAllCompleted,
}

pub struct ScanPhotos {
    scan: photos_core::Scanner,
    repo: Arc<Mutex<photos_core::Repository>>,
}

impl Worker for ScanPhotos {
    type Init = (photos_core::Scanner, Arc<Mutex<photos_core::Repository>>);
    type Input = ScanPhotosInput;
    type Output = ScanPhotosOutput;

    fn init((scan, repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { scan, repo }
    }

    fn update(&mut self, msg: ScanPhotosInput, sender: ComponentSender<Self>) {
        match msg {
            ScanPhotosInput::ScanAll => {
                let result = self.scan_and_add(sender);
                if let Err(e) = result {
                    println!("Failed scan with: {}", e);
                }
            }
        };
    }
}

impl ScanPhotos {
    fn scan_and_add(&mut self, sender: ComponentSender<Self>) -> std::result::Result<(), String> {
        println!("Scanning file system for pictures...");
        let result = self.scan.scan_all().map_err(|e| e.to_string())?;
        self.repo.lock()
            .map_err(|e| e.to_string())?
            .add_all(&result)
            .map_err(|e| e.to_string())?;

        sender.output(ScanPhotosOutput::ScanAllCompleted)
            .map_err(|e| format!("{:?}", e))

    }
}
