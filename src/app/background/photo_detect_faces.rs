// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use relm4::Reducer;
use rayon::prelude::*;
use anyhow::*;
use std::sync::Arc;
use std::result::Result::Ok;
use tracing::{error, info};

use fotema_core::machine_learning::face_extractor::FaceExtractor;

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
};


#[derive(Debug)]
pub enum PhotoDetectFacesInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoDetectFacesOutput {
    // Face detection has started.
    Started,

    // Face detection has completed
    Completed(usize),

}

pub struct PhotoDetectFaces {
    extractor: FaceExtractor,

    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: fotema_core::photo::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoDetectFaces {

    fn detect(&mut self, sender: ComponentSender<Self>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let unprocessed: Vec<fotema_core::photo::model::Picture> = self.repo
            .find_need_face_scan()?
            .into_iter()
            .filter(|pic| pic.path.exists())
            .collect();

        let count = unprocessed.len();
         info!("Found {} photos as candidates for face detection", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoDetectFacesOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PhotoDetectFacesOutput::Started);

        self.progress_monitor.emit(ProgressMonitorInput::Start(TaskName::DetectFaces, count));

        // One thread per CPU core... makes my laptop sluggish and hot... also likes memory.
        // Might need to consider constraining number of CPUs to use less memory or to
        // keep the computer more response while thumbnail generation is going on.
        unprocessed
            .into_iter()
            //.par_iter()
            .for_each(|photo| {
                let result = self.extractor.extract_faces(&photo.picture_id, &photo.path);

                if let Ok(faces) = result {
                    self.repo.add_face_scans(&photo.picture_id, &faces);
                } else {
                    error!("Failed extracting motion photo: {:?}: Photo path: {:?}", result, photo.path);
                };

                self.progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!("Detected faces in {} photos in {} seconds.", count, start.elapsed().as_secs());

        self.progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PhotoDetectFacesOutput::Completed(count));

        Ok(())
    }
}

impl Worker for PhotoDetectFaces {
    type Init = (FaceExtractor, fotema_core::photo::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = PhotoDetectFacesInput;
    type Output = PhotoDetectFacesOutput;

    fn init((extractor, repo, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        PhotoDetectFaces {
            extractor,
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoDetectFacesInput::Start => {
                info!("Extracting photo faces...");

                if let Err(e) = self.detect(sender) {
                    error!("Failed to extract photo faces: {}", e);
                }
            }
        };
    }
}
