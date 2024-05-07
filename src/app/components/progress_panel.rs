// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::shared_state::Reducer;
use relm4::*;

use std::sync::Arc;

use super::progress_monitor::{ProgressMonitor, TaskName, MediaType};


#[derive(Debug)]
pub enum ProgressPanelInput {
    Update(TaskName, f64, usize),
}

/// Shows progress of a background task
pub struct ProgressPanel {
    progress_bar: gtk::ProgressBar,
}

#[relm4::component(pub)]
impl SimpleComponent for ProgressPanel {
    type Init = Arc<Reducer<ProgressMonitor>>;
    type Input = ProgressPanelInput;
    type Output = ();

    view! {
        gtk::ProgressBar {
            set_margin_all: 12,
            set_visible: false,
            set_show_text: true,
            set_pulse_step: 0.05,
        }
    }

    fn init(
        progress_monitor: Self::Init,
        progress_bar: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        progress_monitor.subscribe(&sender.input_sender(),
            |data| ProgressPanelInput::Update(data.task_name, data.fraction(), data.current_count));

        let model = ProgressPanel { progress_bar: progress_bar.clone() };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ProgressPanelInput::Update(task_name, fraction, count) => {
                if count == 0 {
                    self.progress_bar.set_visible(true);
                    match task_name {
                        TaskName::Thumbnail(MediaType::Photo) => {
                            self.progress_bar.set_text(Some("Generating photo thumbnails."));
                        },
                        TaskName::Thumbnail(MediaType::Video) => {
                            self.progress_bar.set_text(Some("Generating video thumbnails."));
                        },
                        TaskName::Transcode => {
                            self.progress_bar.set_text(Some("Converting videos."));
                        },
                        TaskName::Idle => {
                            self.progress_bar.set_text(Some("Idle."));
                        },
                    }
                }

                // Show pulsing for first 20 items so that it catches the eye, then
                // switch to fractional view
                if count < 20 {
                    self.progress_bar.pulse();
                } else if count == 20 {
                    self.progress_bar.set_text(None);
                    self.progress_bar.set_fraction(fraction);
                } else {
                    self.progress_bar.set_fraction(fraction);
                }

                if fraction >= 1.0 {
                    self.progress_bar.set_visible(false);
                    self.progress_bar.set_text(None);
                }
            }
        }
    }
}

