// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Reducer;
use relm4::Worker;

use anyhow::*;

use fotema_core::video::Repository;
use fotema_core::video::Transcoder;
use fotema_core::Visual;
use tracing::{error, info};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::app::components::progress_monitor::{ProgressMonitor, ProgressMonitorInput, TaskName};

use crate::app::SharedState;

#[derive(Debug)]
pub enum VideoTranscodeInput {
    /// Transcode all videos
    Start,
}

#[derive(Debug)]
pub enum VideoTranscodeOutput {
    // Video transcoding has started
    Started,

    // Video transcoding has completed
    Completed,
}

pub struct VideoTranscode {
    // Stop flag
    stop: Arc<AtomicBool>,

    repo: Repository,

    transcoder: Transcoder,

    state: SharedState,

    progress_monitor: Arc<Reducer<ProgressMonitor>>,
}

impl VideoTranscode {
    fn transcode_all(&mut self, sender: &ComponentSender<Self>) -> Result<()> {
        let unprocessed: Vec<Arc<Visual>> = {
            let data = self.state.read();
            data.iter()
                .filter(|&x| x.video_id.is_some())
                .filter(|&x| x.is_transcode_required.is_some_and(|y| y))
                .filter(|&x| x.video_path.as_ref().is_some_and(|y| y.exists()))
                .filter(|&x| !x.video_transcoded_path.as_ref().is_some_and(|y| y.exists()))
                .cloned()
                .collect()
        };

        info!("Found {} videos to transcode", unprocessed.len());

        self.progress_monitor.emit(ProgressMonitorInput::Start(
            TaskName::Transcode,
            unprocessed.len(),
        ));

        let _ = sender.output(VideoTranscodeOutput::Started);

        unprocessed
            .iter()
            .take_while(|_| !self.stop.load(Ordering::Relaxed))
            .for_each(|visual| {
                let video_id = visual.video_id.expect("Must have video_id");
                let video_path = visual.video_path.as_ref().expect("Must have video_path");

                let result = self
                    .transcoder
                    .transcode(video_id, video_path)
                    .with_context(|| format!("Video path: {:?}", video_path));

                if let std::result::Result::Ok(ref transcode_path) = result {
                    if let Err(e) = self.repo.add_transcode(video_id, transcode_path) {
                        error!("Failed adding transcode path: {:?}", e);
                    }
                } else if let Err(ref e) = result {
                    error!("Failed transcoding: {:?}", e);
                }

                self.progress_monitor.emit(ProgressMonitorInput::Advance);
            });

        self.progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(VideoTranscodeOutput::Completed);

        Ok(())
    }
}

impl Worker for VideoTranscode {
    type Init = (
        Arc<AtomicBool>,
        SharedState,
        Repository,
        Transcoder,
        Arc<Reducer<ProgressMonitor>>,
    );
    type Input = VideoTranscodeInput;
    type Output = VideoTranscodeOutput;

    fn init(
        (stop, state, repo, transcoder, progress_monitor): Self::Init,
        _sender: ComponentSender<Self>,
    ) -> Self {
        Self {
            stop,
            state,
            repo,
            transcoder,
            progress_monitor,
        }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoTranscodeInput::Start => {
                info!("Transcoding all incompatible videos");

                if let Err(e) = self.transcode_all(&sender) {
                    error!("Failed to transcode photo: {}", e);
                }
            }
        };
    }
}
