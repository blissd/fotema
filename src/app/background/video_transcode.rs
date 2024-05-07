// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use relm4::Reducer;

use anyhow::*;

use fotema_core::video::Repository;
use fotema_core::video::Transcoder;
use fotema_core::Visual;
use tracing::{event, Level};

use crate::app::components::progress_monitor::{
    ProgressMonitor,
    ProgressMonitorInput,
    TaskName,
};

use std::sync::Arc;

use crate::app::SharedState;

#[derive(Debug)]
pub enum VideoTranscodeInput {
    /// Transcode all videos
    All,
}

#[derive(Debug)]
pub enum VideoTranscodeOutput {
    // Thumbnail generation has started for a given number of images.
    Started,

    // Thumbnail generation has completed
    Completed,

}

pub struct VideoTranscode {
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

        self.progress_monitor
            .emit(ProgressMonitorInput::Start(TaskName::Transcode, unprocessed.len()));

        let _ = sender.output(VideoTranscodeOutput::Started);

        unprocessed
            .iter()
            .for_each(|visual| {
                let video_id = visual.video_id.expect("Must have video_id");
                let video_path = visual.video_path.as_ref().expect("Must have video_path");

                let result = self.transcoder.transcode(video_id, &video_path);

                if let std::result::Result::Ok(ref transcode_path) = result {
                    if let Err(e) = self.repo.add_transcode(video_id, transcode_path) {
                        event!(Level::ERROR, "Failed adding transcode path: {:?}", e);
                    }
                }

                self.progress_monitor.emit(ProgressMonitorInput::Advance);

            });

        self.progress_monitor.emit(ProgressMonitorInput::Complete);

        let _ = sender.output(VideoTranscodeOutput::Completed);

        Ok(())
    }
}

impl Worker for VideoTranscode {
    type Init = (SharedState, Repository, Transcoder, Arc<Reducer<ProgressMonitor>>);
    type Input = VideoTranscodeInput;
    type Output = VideoTranscodeOutput;

    fn init((state, repo, transcoder, progress_monitor): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { state, repo, transcoder, progress_monitor }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoTranscodeInput::All => {
                event!(Level::INFO, "Transcoding all incompatible videos");

                if let Err(e) = self.transcode_all(&sender) {
                    event!(Level::ERROR, "Failed to transcode photo: {}", e);
                }
            },
        };
    }
}
