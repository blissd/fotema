// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;
use anyhow::*;

use fotema_core::video::Repository;
use fotema_core::VisualId;
use fotema_core::video::Transcoder;

use crate::app::SharedState;

#[derive(Debug)]
pub enum VideoTranscodeInput {
    /// Transcode one video
    One(VisualId),
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
}

impl VideoTranscode {

    fn transcode_one(&mut self, visual_id: VisualId, sender: &ComponentSender<Self>) -> Result<()> {

        let visual = {
            let data = self.state.read();
            data.iter().find(|&x| x.visual_id == visual_id).cloned()
        };

        let Some(visual) = visual else {
            return Err(anyhow!("Visual not found: {}", visual_id));
        };

        let Some(video_id) = visual.video_id else {
            return Err(anyhow!("Visual does not have video_id: {}", visual_id));
        };

        let Some(ref video_path) = visual.video_path else {
            return Err(anyhow!("Visual does not have video_path: {}", visual_id));
        };

        let result = self.transcoder.transcode(video_id, &video_path);
        if let std::result::Result::Ok(ref transcode_path) = result {
            self.repo.add_transcode(video_id, transcode_path);
        }

        if let Err(e) = sender.output(VideoTranscodeOutput::Completed) {
            println!("Failed sending VideoTranscodeOutput::Completed: {:?}", e);
        }

        Ok(())
    }
}

impl Worker for VideoTranscode {
    type Init = (SharedState, Repository, Transcoder);
    type Input = VideoTranscodeInput;
    type Output = VideoTranscodeOutput;

    fn init((state, repo, transcoder): Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { state, repo, transcoder }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            VideoTranscodeInput::One(visual_id) => {
                println!("Transcoding item with visual_id: {}", visual_id);

                if let Err(e) = self.transcode_one(visual_id, &sender) {
                    println!("Failed to transcode photo: {}", e);
                }
            },
        };
    }
}
