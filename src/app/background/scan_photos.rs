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
    ScanAllCompleted(Vec<photos_core::scanner::Picture>),
}

pub struct ScanPhotos {
    controller: Arc<Mutex<photos_core::Controller>>,
}

impl Worker for ScanPhotos {
    type Init = Arc<Mutex<photos_core::Controller>>;
    type Input = ScanPhotosInput;
    type Output = ScanPhotosOutput;

    fn init(controller: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { controller }
    }

    fn update(&mut self, msg: ScanPhotosInput, sender: ComponentSender<Self>) {
        match msg {
            ScanPhotosInput::ScanAll => {
                println!("Scanning file system for pictures...");
                let start_at = std::time::SystemTime::now();
                let result = self.controller.lock().expect("lock mutex").scan_and_add();
                let end_at = std::time::SystemTime::now();

                if let Ok(pics) = result {
                    let duration = end_at.duration_since(start_at).unwrap_or(std::time::Duration::new(0, 0));
                    println!("Scanned some items in {} seconds",duration.as_secs());
                    //let _ = sender.output(ScanPhotosOutput::ScanAllCompleted(pics));
                } else {
                    println!("Failed scanning: {:?}", result);
                }
            }
        };
    }
}
