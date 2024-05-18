// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use fotema_core::Visual;
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

use std::sync::Arc;

use tracing::{event, Level};

#[derive(Debug)]
pub enum OnePhotoInput {
    // View an item.
    View(Arc<Visual>),

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,

    // Transcode all incompatible videos
    TranscodeAll,

    MuteToggle,

    PlayToggle,

    VideoEnded,
}

#[derive(Debug)]
pub enum OnePhotoOutput {
    TranscodeAll,

    PhotoShown(VisualId, glycin::ImageInfo),

    VideoShown(VisualId),
}

pub struct OnePhoto {
    picture: gtk::Picture,

    video: Option<gtk::MediaFile>,

    video_controls: gtk::Box,

    play_button: gtk::Button,

    mute_button: gtk::Button,

    transcode_button: gtk::Button,

    transcode_status: adw::StatusPage,

    transcode_progress: Controller<ProgressPanel>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for OnePhoto {
    type Init = Arc<Reducer<ProgressMonitor>>;
    type Input = OnePhotoInput;
    type Output = OnePhotoOutput;

    view! {
        // FIXME should probably be a gtk::Stack because visibility of picture and transcode_status
        // is mutually exclusive.
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Overlay {
                set_vexpand: true,
                set_halign: gtk::Align::Center,

                #[local_ref]
                add_overlay =  &video_controls -> gtk::Box {
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::End,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_margin_all: 18,
                    set_spacing: 12,

                    #[local_ref]
                    play_button -> gtk::Button {
                        set_icon_name: "play-symbolic",
                        add_css_class: "circular",
                        add_css_class: "osd",
                        connect_clicked => OnePhotoInput::PlayToggle,
                    },

                    #[local_ref]
                    mute_button -> gtk::Button {
                        set_icon_name: "audio-volume-muted-symbolic",
                        add_css_class: "circular",
                        add_css_class: "osd",
                        connect_clicked => OnePhotoInput::MuteToggle,
                    },
                },

                #[wrap(Some)]
                //#[local_ref]
                set_child = &gtk::Box {
                    #[local_ref]
                    picture -> gtk::Picture {
                    }
                },
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
        transcode_progress_monitor: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let picture = gtk::Picture::new();

        let video_controls = gtk::Box::new(gtk::Orientation::Horizontal, 18);

        let play_button = gtk::Button::new();

        let mute_button = gtk::Button::new();

        let transcode_button = gtk::Button::new();

        let transcode_progress = ProgressPanel::builder()
            .launch(transcode_progress_monitor.clone())
            .detach();

        let transcode_status = adw::StatusPage::new();


        let model = OnePhoto {
            picture: picture.clone(),
            video: None,
            video_controls: video_controls.clone(),
            play_button: play_button.clone(),
            mute_button: mute_button.clone(),
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
                self.video = None;
                self.picture.set_paintable(None::<&gdk::Paintable>);
            },
            OnePhotoInput::View(visual) => {
                event!(Level::INFO, "Showing item for {}", visual.visual_id);

                let visual_path = visual.picture_path.clone()
                    .or_else(|| visual.video_path.clone())
                    .expect("Must have path");

                self.picture.set_paintable(None::<&gdk::Paintable>);
                self.video = None;

                // clear orientation transformation css classes
                for orient in PictureOrientation::iter() {
                    self.picture.remove_css_class(orient.as_ref());
                }

                if visual.is_photo_only() {
                    self.picture.set_visible(true);
                    self.transcode_status.set_visible(false);
                    self.video_controls.set_visible(false);

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

                    let _ = sender.output(OnePhotoOutput::PhotoShown(visual.visual_id.clone(), image.info().clone()));
                } else { // video or motion photo
                    let is_transcoded = visual.video_transcoded_path.as_ref().is_some_and(|x| x.exists());

                    if visual.is_transcode_required.is_some_and(|x| x) && !is_transcoded {
                        self.picture.set_visible(false);
                        self.transcode_status.set_visible(true);
                        self.video_controls.set_visible(false);
                    } else {
                        self.picture.set_visible(true);
                        self.transcode_status.set_visible(false);
                        self.video_controls.set_visible(true);

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

                        let video = gtk::MediaFile::for_filename(video_path);
                        if visual.is_motion_photo() {
                           self.mute_button.set_icon_name("audio-volume-muted-symbolic");
                           video.set_muted(true);
                           video.set_loop(true);
                        } else {
                           self.mute_button.set_icon_name("multimedia-volume-control-symbolic");
                           video.set_muted(false);
                           video.set_loop(false);
                           let sender = sender.clone();
                           video.connect_ended_notify(move |_| sender.input(OnePhotoInput::VideoEnded));
                        }
                        // Always set volume to 1 because muting sometimes seems to disable
                        // the volume permanently even when set to false.
                        video.set_volume(1.0);

                        video.play();
                        self.play_button.set_icon_name("pause-symbolic");

                        self.video = Some(video);
                        self.picture.set_paintable(self.video.as_ref());
                        let _ = sender.output(OnePhotoOutput::VideoShown(visual.visual_id.clone()));
                    }
                }
            },
            OnePhotoInput::MuteToggle => {
                if let Some(ref video) = self.video {
                    if video.is_muted() {
                        self.mute_button.set_icon_name("multimedia-volume-control-symbolic");
                        video.set_muted(false);
                    } else {
                        self.mute_button.set_icon_name("audio-volume-muted-symbolic");
                        video.set_muted(true);
                    }
                }
            },
            OnePhotoInput::PlayToggle => {
                if let Some(ref video) = self.video {
                    if video.is_ended() {
                        video.seek(0);

                        // I'd like to just set the play_button icon to pause-symbolic and
                        // play the video. However, if we just call play, then the play button icon
                        // doesn't update and stays as the replay icon.
                        //
                        // Playing, pausing, and sending a new message seems
                        // to work around that.
                        video.play();
                        video.pause();
                        sender.input(OnePhotoInput::PlayToggle);
                    } else if video.is_playing() {
                        video.pause();
                        self.play_button.set_icon_name("play-symbolic");
                    } else { // is paused
                        video.play();
                        self.play_button.set_icon_name("pause-symbolic");
                    }
                }
            },
            OnePhotoInput::VideoEnded => {
                self.play_button.set_icon_name("arrow-circular-top-left-symbolic");
            },
            OnePhotoInput::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                self.transcode_button.set_visible(false);
                let _ = sender.output(OnePhotoOutput::TranscodeAll);
            },
        }
    }
}
