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
                println!("Scanning file system for pictures...");
                let result = self.scan.scan_all().unwrap();
                self.repo.lock().expect("mutex lock").add_all(&result);
                sender.output(ScanPhotosOutput::ScanAllCompleted);
            }
        };
    }
}
