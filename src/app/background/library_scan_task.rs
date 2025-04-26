// SPDX-FileCopyrightText: © 2024-2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::Worker;
use relm4::prelude::*;
use tracing::{error, info};
use fotema_core::{Scanner, ScannedFile};
use fotema_core::photo::Repository as PhotoRepository;
use fotema_core::video::Repository as VideoRepository;
use itertools::{Itertools, Either};

#[derive(Debug)]
pub enum LibraryScanTaskInput {
    Start,
}

#[derive(Debug)]
pub enum LibraryScanTaskOutput {
    Started,
    Completed,
}

pub struct LibraryScanTask {
    scan: Scanner,
    photo_repo: PhotoRepository,
    video_repo: VideoRepository,
}

impl Worker for LibraryScanTask {
    type Init = (Scanner, PhotoRepository, VideoRepository);
    type Input = LibraryScanTaskInput;
    type Output = LibraryScanTaskOutput;

    fn init((scan, photo_repo, video_repo): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { scan, photo_repo, video_repo }
    }

    fn update(&mut self, msg: LibraryScanTaskInput, sender: ComponentSender<Self>) {
        match msg {
            LibraryScanTaskInput::Start => {
                let result = self.scan_and_add(sender);
                if let Err(e) = result {
                    error!("Failed scan with: {}", e);
                }
            }
        };
    }
}

impl LibraryScanTask {
    fn scan_and_add(&mut self, sender: ComponentSender<Self>) -> std::result::Result<(), String> {
        let start = std::time::Instant::now();

        sender
            .output(LibraryScanTaskOutput::Started)
            .map_err(|e| format!("{:?}", e))?;

        info!("Scanning file system for pictures...");

        let result = self.scan.scan_all().map_err(|e| e.to_string())?;

        let (photos, videos) = result.into_iter().partition_map(|scanned_file|
            match scanned_file {
                f @ ScannedFile::Photo(_) => Either::Left(f),
                f @ ScannedFile::Video(_) => Either::Right(f),
            });

        self.photo_repo.add_all(&photos).map_err(|e| e.to_string())?;
        self.video_repo.add_all(&videos).map_err(|e| e.to_string())?;

        info!(
            "Scanned {} photos and {} videos in {} seconds.",
            photos.len(),
            videos.len(),
            start.elapsed().as_secs()
        );

        sender
            .output(LibraryScanTaskOutput::Completed)
            .map_err(|e| format!("{:?}", e))
    }
}
