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
use std::path::PathBuf;
use tracing::{error, info};
use futures::executor::block_on;

use fotema_core::machine_learning::face_recognizer::FaceRecognizer;
use fotema_core::people;
use fotema_core::PersonId;
use fotema_core::people::model::DetectedFace;
use fotema_core::photo::PictureId;

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
};


#[derive(Debug)]
pub enum PhotoRecognizeFacesInput {
    Start,
}

#[derive(Debug)]
pub enum PhotoRecognizeFacesOutput {
    // Face recognition has started.
    Started,

    // Face recognition has completed
    Completed(usize),

}

#[derive(Clone)]
pub struct PhotoRecognizeFaces {
    // Danger! Don't hold the repo mutex for too long as it blocks viewing images.
    repo: people::Repository,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl PhotoRecognizeFaces {

    fn recognize(&self, sender: ComponentSender<Self>) -> Result<()>
     {
        let start = std::time::Instant::now();

        let people: Vec<(PersonId, DetectedFace)> = self.repo
            .find_people_for_recognition()?
            .into_iter()
            .collect();

        let count = people.len();
         info!("Found {} people as candidates for face recognition", count);

        // Short-circuit before sending progress messages to stop
        // banner from appearing and disappearing.
        if count == 0 {
            let _ = sender.output(PhotoRecognizeFacesOutput::Completed(count));
            return Ok(());
        }

        let _ = sender.output(PhotoRecognizeFacesOutput::Started);

        self.progress_monitor.emit(ProgressMonitorInput::Start(TaskName::RecognizeFaces, count));

        people
            .into_iter()
            .for_each(|(person_id, person_face)| {
                // FIXME unwrap
               // let mut recognizer = FaceRecognizer::build(&person_face).unwrap();

                // FIXME unwrap
                let unknown_faces = self.repo.find_unknown_faces_for_person(person_id).unwrap();
                info!("Recognizing person {} against {} unknown faces.", person_id, unknown_faces.len());

                unknown_faces
                    .into_par_iter()
                    .for_each(|unknown_face| {
                        let mut recognizer = FaceRecognizer::build(&person_face).unwrap();
                        let is_match = recognizer.recognize(&unknown_face);
                        if let Ok(true) = is_match {
                            let mut repo = self.repo.clone();
                            let result = repo.mark_as_person_unconfirmed(unknown_face.face_id, person_id);
                            if let Err(e) = result {
                                error!("Failed marking face {} as person: {:?}", unknown_face.face_id, e);
                            }
                        }
                    });

/*
                if result.is_err() {
                    error!("Failed detecting faces: Photo path: {:?}. Error: {:?}", path, result);
                    let _ = repo.mark_face_scan_broken(&picture_id);
                }*/

                self.progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        info!("Recognized people in {} seconds.", start.elapsed().as_secs());

        self.progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(PhotoRecognizeFacesOutput::Completed(count));

        Ok(())
    }
}

impl Worker for PhotoRecognizeFaces {
    type Init = (people::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = PhotoRecognizeFacesInput;
    type Output = PhotoRecognizeFacesOutput;

    fn init((repo, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self  {
        PhotoRecognizeFaces {
            repo,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoRecognizeFacesInput::Start => {
                info!("Recognizing photo faces...");
                let this = self.clone();

                // Avoid runtime panic from calling block_on
                rayon::spawn(move || {
                    if let Err(e) = this.recognize(sender) {
                        error!("Failed to recognize photo faces: {}", e);
                    }
                });
            }
        };
    }
}
