// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use fotema_core::visual::model::PictureOrientation;
use strum::IntoEnumIterator;
use relm4::gtk;
use relm4::adw::gdk;
use relm4::gtk::gio;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;
use glycin;

use crate::app::components::progress_monitor::ProgressMonitor;
use crate::app::components::progress_panel::ProgressPanel;
use crate::app::SharedState;

use std::sync::Arc;

use tracing::{event, Level};

#[derive(Debug)]
pub enum OnePhotoInput {
    // View an item.
    View(VisualId),

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,

    // Transcode all incompatible videos
    TranscodeAll,
}

#[derive(Debug)]
pub enum OnePhotoOutput {
    TranscodeAll,

    PhotoShown(VisualId, glycin::ImageInfo),

    VideoShown(VisualId),
}

pub struct OnePhoto {
    state: SharedState,

    picture: gtk::Picture,

    transcode_button: gtk::Button,

    transcode_status: adw::StatusPage,

    transcode_progress: Controller<ProgressPanel>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for OnePhoto {
    type Init = (SharedState, Arc<Reducer<ProgressMonitor>>);
    type Input = OnePhotoInput;
    type Output = OnePhotoOutput;

    view! {
        // FIXME should probably be a gtk::Stack because visibility of picture and transcode_status
        // is mutually exclusive.
        gtk::Box {
            set_halign: gtk::Align::Center,

            #[local_ref]
            picture -> gtk::Picture {
            },

            #[local_ref]
            transcode_status -> adw::StatusPage {
                set_visible: false,
                set_icon_name: Some("playback-error-symbolic"),
                set_description: Some("This video must be converted before it can be played.\nThis only needs to happen once, but it takes a while to convert a video."),

                #[wrap(Some)]
                set_child = &adw::Clamp {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_maximum_size: 400,

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[local_ref]
                        transcode_button -> gtk::Button {
                            set_label: "Convert all incompatible videos",
                            add_css_class: "suggested-action",
                            add_css_class: "pill",
                            connect_clicked => OnePhotoInput::TranscodeAll,
                        },

                        model.transcode_progress.widget(),
                    }
                }
            }
        }
    }

    async fn init(
        (state, transcode_progress_monitor): Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let picture = gtk::Picture::new();

        let transcode_button = gtk::Button::new();

        let transcode_progress = ProgressPanel::builder()
            .launch(transcode_progress_monitor.clone())
            .detach();

        let transcode_status = adw::StatusPage::new();


        let model = OnePhoto {
            state,
            picture: picture.clone(),
            transcode_button: transcode_button.clone(),
            transcode_status: transcode_status.clone(),
            transcode_progress,
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            OnePhotoInput::Hidden => {
                self.picture.set_paintable(None::<&gdk::Paintable>);
            },
            OnePhotoInput::View(visual_id) => {
                event!(Level::INFO, "Showing item for {}", visual_id);

                let result = {
                    let data = self.state.read();
                    data.iter().find(|&x| x.visual_id == visual_id).cloned()
                };

                let visual = if let Some(v) = result {
                    v
                } else {
                    event!(Level::ERROR, "Failed loading visual item: {:?}", result);
                    return;
                };

                let visual_path = visual.picture_path.clone()
                    .or_else(|| visual.video_path.clone())
                    .expect("Must have path");

                self.picture.set_paintable(None::<&gdk::Paintable>);

                // clear orientation transformation css classes
                for orient in PictureOrientation::iter() {
                    self.picture.remove_css_class(orient.as_ref());
                }

                if visual.is_photo_only() {
                    self.picture.set_visible(true);
                    self.transcode_status.set_visible(false);

                    // Apply a CSS transformation to respect the EXIF orientation
                    let orientation = visual.picture_orientation
                        .unwrap_or(PictureOrientation::North);
                    self.picture.add_css_class(orientation.as_ref());

                    let file = gio::File::for_path(visual_path.clone());
                    let image_result = glycin::Loader::new(file).load().await;

                    let image = if let Ok(image) = image_result {
                        image
                    } else {
                        event!(Level::ERROR, "Failed loading image: {:?}", image_result);
                        return;
                    };

                    let frame = if let Ok(frame) = image.next_frame().await {
                        frame
                    } else {
                        event!(Level::ERROR, "Failed getting image frame");
                        return;
                    };

                    let texture = frame.texture;

                    self.picture.set_paintable(Some(&texture));

                    let _ = sender.output(OnePhotoOutput::PhotoShown(visual_id, image.info().clone()));
                } else { // video or motion photo
                    let is_transcoded = visual.video_transcoded_path.as_ref().is_some_and(|x| x.exists());

                    if visual.is_transcode_required.is_some_and(|x| x) && !is_transcoded {
                        self.picture.set_visible(false);
                        self.transcode_status.set_visible(true);
                    } else {
                        self.picture.set_visible(true);
                        self.transcode_status.set_visible(false);


                        // if a video is transcoded then the rotation transformation will
                        // already have been applied.
                        if !is_transcoded {
                            // Apply a CSS transformation to respect the display matrix rotation
                            let orientation = visual.video_orientation
                                .unwrap_or(PictureOrientation::North);
                            self.picture.add_css_class(orientation.as_ref());
                        }

                        let video_path = visual.video_transcoded_path.clone()
                            .filter(|x| x.exists())
                            .or_else(|| visual.video_path.clone())
                            .expect("must have video path");

                        let media_file = gtk::MediaFile::for_filename(video_path);
                        self.picture.set_paintable(Some(&media_file));

                        if visual.is_motion_photo() {
                           //media_file.set_muted(true);
                           media_file.set_loop(true);
                        } else {
                           //media_file.set_muted(false);
                           media_file.set_loop(false);
                        }

                        media_file.play();
                        let _ = sender.output(OnePhotoOutput::VideoShown(visual_id));
                    }
                }
            },
            OnePhotoInput::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                self.transcode_button.set_visible(false);
                let _ = sender.output(OnePhotoOutput::TranscodeAll);
            },
        }
    }
}
