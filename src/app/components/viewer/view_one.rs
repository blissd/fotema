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
use chrono::TimeDelta;

use crate::app::components::progress_monitor::ProgressMonitor;
use crate::app::components::progress_panel::ProgressPanel;
use crate::fl;
use fotema_core::people;
use super::face_thumbnails::{FaceThumbnails, FaceThumbnailsInput};

use std::sync::Arc;

use tracing::{event, Level};

const TEN_SECS_IN_MICROS: i64 = 10_000_000;
const FIFTEEN_SECS_IN_MICROS: i64 = 15_000_000;

#[derive(Debug)]
pub enum ViewOneInput {
    // View an item.
    View(Arc<Visual>),

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,

    /// Refresh view. Probably because face thumbnails have updated elsewhere.
    /// FIXME not sure I like view_nav.rs being responsible for ignoring/restoring faces.
    Refresh,

    // Transcode all incompatible videos
    TranscodeAll,

    MuteToggle,

    PlayToggle,


    SkipBackwards,

    SkipForward,

    // Signal when video ends
    VideoEnded,

    // Constantly sent during video playback so we can update the timestamp.
    VideoTimestamp,

    // Video has been "prepared", so duration should be available
    VideoPrepared,
}

#[derive(Debug)]
pub enum ViewOneOutput {
    TranscodeAll,

    PhotoShown(VisualId, glycin::ImageInfo),

    VideoShown(VisualId),
}

pub struct ViewOne {
    picture: gtk::Picture,

    video: Option<gtk::MediaFile>,

    is_transcode_required: bool,

    play_button: gtk::Button,

    mute_button: gtk::Button,

    skip_backwards: gtk::Button,

    skip_forward: gtk::Button,

    video_timestamp: gtk::Label,

    transcode_button: gtk::Button,

    transcode_status: adw::StatusPage,

    transcode_progress: Controller<ProgressPanel>,

    broken_status: adw::StatusPage,

    face_thumbnails: AsyncController<FaceThumbnails>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for ViewOne {
    type Init = (people::Repository, Arc<Reducer<ProgressMonitor>>);
    type Input = ViewOneInput;
    type Output = ViewOneOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_vexpand: true,
            set_hexpand: true,

            gtk::Overlay {
                set_vexpand: true,
                set_halign: gtk::Align::Center,

                // video_controls
                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_spacing: 12,
                    set_halign: gtk::Align::Center,
                    set_valign: gtk::Align::End,

                    #[watch]
                    set_visible: model.is_video_controls_visible(),

                    gtk::Frame {
                        set_halign: gtk::Align::Center,
                        add_css_class: "osd",

                        #[wrap(Some)]
                        #[local_ref]
                        set_child = &video_timestamp -> gtk::Label{
                            set_halign: gtk::Align::Center,
                            add_css_class: "photo-grid-month-label",
                        },
                    },
                    gtk::Box {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::End,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_start: 18,
                        set_margin_end: 18,
                        set_margin_bottom: 18,
                        set_spacing: 12,

                        #[local_ref]
                        skip_backwards -> gtk::Button {
                            set_icon_name: "skip-backwards-10-symbolic",
                            add_css_class: "circular",
                            add_css_class: "osd",
                            set_tooltip_text: Some(&fl!("viewer-skip-backwards-10-seconds", "tooltip")),
                            connect_clicked => ViewOneInput::SkipBackwards,
                        },

                        #[local_ref]
                        play_button -> gtk::Button {
                            set_icon_name: "play-symbolic",
                            add_css_class: "circular",
                            add_css_class: "osd",
                            set_tooltip_text: Some(&fl!("viewer-play", "tooltip")),
                            connect_clicked => ViewOneInput::PlayToggle,
                        },

                        #[local_ref]
                        skip_forward -> gtk::Button {
                            set_icon_name: "skip-forward-10-symbolic",
                            add_css_class: "circular",
                            add_css_class: "osd",
                            set_tooltip_text: Some(&fl!("viewer-skip-forward-10-seconds", "tooltip")),
                            connect_clicked => ViewOneInput::SkipForward,
                        },

                        #[local_ref]
                        mute_button -> gtk::Button {
                            set_icon_name: "audio-volume-muted-symbolic",
                            set_margin_start: 36,
                            add_css_class: "circular",
                            add_css_class: "osd",
                            set_tooltip_text: Some(&fl!("viewer-mute", "tooltip")),
                            connect_clicked => ViewOneInput::MuteToggle,
                        },
                    },
                },

                add_overlay = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Start,
                    set_valign: gtk::Align::End,
                    set_margin_all: 8,
                    container_add: model.face_thumbnails.widget(),
                },

                #[wrap(Some)]
                set_child = &gtk::Box {
                    #[local_ref]
                    picture -> gtk::Picture {
                    }
                },
            },

            #[local_ref]
            transcode_status -> adw::StatusPage {
                set_valign: gtk::Align::Start,
                set_vexpand: true,

                set_visible: false,
                set_icon_name: Some("playback-error-symbolic"),
                set_description: Some(&fl!("viewer-convert-all-description")),

                #[wrap(Some)]
                set_child = &adw::Clamp {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_maximum_size: 400,

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[local_ref]
                        transcode_button -> gtk::Button {
                            set_label: &fl!("viewer-convert-all-button"),
                            add_css_class: "suggested-action",
                            add_css_class: "pill",
                            connect_clicked => ViewOneInput::TranscodeAll,
                        },

                        model.transcode_progress.widget(),
                    }
                }
            },

            #[local_ref]
            broken_status -> adw::StatusPage {
                set_valign: gtk::Align::Start,
                set_vexpand: true,

                set_visible: false,
                set_icon_name: Some("sad-computer-symbolic"),
            }
        }
    }

    async fn init(
        (people_repo, transcode_progress_monitor): Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let picture = gtk::Picture::new();

        let play_button = gtk::Button::new();

        let mute_button = gtk::Button::new();

        let skip_backwards = gtk::Button::new();

        let skip_forward = gtk::Button::new();

        let video_timestamp = gtk::Label::new(None);

        let transcode_button = gtk::Button::new();

        let transcode_progress = ProgressPanel::builder()
            .launch(transcode_progress_monitor.clone())
            .detach();

        let transcode_status = adw::StatusPage::new();

        let broken_status = adw::StatusPage::new();

        let face_thumbnails = FaceThumbnails::builder()
            .launch(people_repo)
            .detach();

        let model = ViewOne {
            picture: picture.clone(),
            video: None,
            is_transcode_required: false,
            play_button: play_button.clone(),
            mute_button: mute_button.clone(),
            skip_backwards: skip_backwards.clone(),
            skip_forward: skip_forward.clone(),
            video_timestamp: video_timestamp.clone(),
            transcode_button: transcode_button.clone(),
            transcode_status: transcode_status.clone(),
            transcode_progress,
            broken_status: broken_status.clone(),
            face_thumbnails,
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            ViewOneInput::Hidden => {
                self.video = None;
                self.picture.set_paintable(None::<&gdk::Paintable>);
                self.face_thumbnails.emit(FaceThumbnailsInput::Hide);
            },
            ViewOneInput::View(visual) => {
                event!(Level::INFO, "Showing item for {}", visual.visual_id);

                self.picture.set_visible(false);
                self.transcode_status.set_visible(false);
                self.broken_status.set_visible(false);
                self.is_transcode_required = false;

                let visual_path = visual.picture_path.as_ref()
                    .or_else(|| visual.video_path.as_ref());

                let Some(visual_path) = visual_path else {
                    if visual.is_video_only() {
                        self.broken_status.set_icon_name(Some("item-missing-symbolic"));
                    } else {
                        self.broken_status.set_icon_name(Some("image-missing-symbolic"));
                    }
                    self.broken_status.set_description(Some(&fl!("viewer-error-missing-path")));
                    self.broken_status.set_visible(true);
                    return;
                };

                if !visual_path.exists() {
                    if visual.is_video_only() {
                        self.broken_status.set_icon_name(Some("item-missing-symbolic"));
                    } else {
                        self.broken_status.set_icon_name(Some("image-missing-symbolic"));
                    }
                    self.broken_status.set_description(Some(&fl!("viewer-error-missing-file",
                        file_name = visual_path.to_string_lossy())));
                    self.broken_status.set_visible(true);
                    return;
                }

                self.picture.set_paintable(None::<&gdk::Paintable>);
                self.video = None;

                // clear orientation transformation css classes
                for orient in PictureOrientation::iter() {
                    self.picture.remove_css_class(orient.as_ref());
                }

                if visual.is_photo_only() {
                    // Apply a CSS transformation to respect the EXIF orientation
                    // NOTE: don't use Glycin to apply the transformation here because it is
                    // too slow.
                    let orientation = visual.picture_orientation
                        .unwrap_or(PictureOrientation::North);
                    self.picture.add_css_class(orientation.as_ref());

                    let file = gio::File::for_path(visual_path);

                    let mut loader = glycin::Loader::new(file);
                    loader.sandbox_selector(glycin::SandboxSelector::FlatpakSpawn);
                    loader.apply_transformations(false);

                    let image = loader.load().await;

                    let Ok(image) = image else {
                        event!(Level::ERROR, "Failed loading image: {:?}", image);
                        self.broken_status.set_icon_name(Some("sad-computer-symbolic"));
                        self.broken_status.set_description(Some(&fl!("viewer-error-failed-to-load")));
                        self.broken_status.set_visible(true);
                        return;
                    };

                    let frame = image.next_frame().await;
                    let Ok(frame) = frame else {
                        event!(Level::ERROR, "Failed getting image frame: {:?}", frame);
                        self.broken_status.set_icon_name(Some("sad-computer-symbolic"));
                        self.broken_status.set_description(Some(&fl!("viewer-error-failed-to-load")));
                        self.broken_status.set_visible(true);
                        return;
                    };

                    let texture = frame.texture();

                    self.picture.set_paintable(Some(&texture));
                    self.picture.set_visible(true);

                    let _ = sender.output(ViewOneOutput::PhotoShown(visual.visual_id.clone(), image.info().clone()));
                } else { // video or motion photo
                    let is_transcoded = visual.video_transcoded_path.as_ref().is_some_and(|x| x.exists());
                    self.is_transcode_required = visual.is_transcode_required.is_some_and(|x| x) && !is_transcoded;

                    if self.is_transcode_required {
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

                        let video_path = visual.video_transcoded_path.as_ref()
                            .filter(|x| x.exists())
                            .or_else(|| visual.video_path.as_ref())
                            .filter(|x| x.exists())
                            .or_else(|| visual.motion_photo_video_path.as_ref())
                            .expect("must have video path");

                        let video = gtk::MediaFile::for_filename(video_path);
                        if visual.is_motion_photo() {
                           self.mute_button.set_icon_name("audio-volume-muted-symbolic");
                           self.skip_backwards.set_visible(false);
                           self.skip_forward.set_visible(false);
                           self.video_timestamp.set_visible(false);
                           video.set_muted(true);
                           video.set_loop(true);
                        } else {
                            self.mute_button.set_icon_name("multimedia-volume-control-symbolic");
                            self.skip_backwards.set_visible(true);
                            self.skip_forward.set_visible(true);
                            self.skip_forward.set_sensitive(true);
                            self.video_timestamp.set_visible(true);

                            // Instead of video.set_muted(false), we must mute and then
                            // send a message to unmute. This seems to work around the problem
                            // of videos staying muted after viewing muting and unmuting.
                            video.set_muted(true);
                            sender.input(ViewOneInput::MuteToggle);

                            video.set_loop(false);

                            let sender1 = sender.clone();
                            let sender2 = sender.clone();
                            let sender3 = sender.clone();
                            video.connect_ended_notify(move |_| sender1.input(ViewOneInput::VideoEnded));
                            video.connect_timestamp_notify(move |_| sender2.input(ViewOneInput::VideoTimestamp));
                            video.connect_prepared_notify(move |_| sender3.input(ViewOneInput::VideoPrepared));
                        }

                        video.play();
                        self.play_button.set_icon_name("pause-symbolic");

                        self.video = Some(video);
                        self.picture.set_paintable(self.video.as_ref());
                        self.picture.set_visible(true);
                        let _ = sender.output(ViewOneOutput::VideoShown(visual.visual_id.clone()));
                    }
                }

                // Overlay faces in picture, but only if transcode status page not visible.
                if !self.transcode_status.is_visible() {
                    if let Some(ref picture_id) = visual.picture_id {
                        self.face_thumbnails.emit(FaceThumbnailsInput::View(*picture_id));
                    } else {
                        self.face_thumbnails.emit(FaceThumbnailsInput::Hide);
                    }
                } else {
                    self.face_thumbnails.emit(FaceThumbnailsInput::Hide);
                }
            },
            ViewOneInput::VideoPrepared => {
                // Video details, like duration, aren't available until the video
                // has been prepared.
                if let Some(ref video) = self.video {
                    if video.duration() < FIFTEEN_SECS_IN_MICROS {
                        self.skip_backwards.set_visible(false);
                        self.skip_forward.set_visible(false);
                    } else {
                        self.skip_backwards.set_visible(true);
                        self.skip_forward.set_visible(true);
                    }
                }
            },
            ViewOneInput::MuteToggle => {
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
            ViewOneInput::PlayToggle => {
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
                        sender.input(ViewOneInput::PlayToggle);
                    } else if video.is_playing() {
                        video.pause();
                        self.play_button.set_icon_name("play-symbolic");
                    } else { // is paused
                        video.play();
                        self.play_button.set_icon_name("pause-symbolic");
                        self.skip_forward.set_sensitive(true);
                    }
                }
            },
            ViewOneInput::SkipBackwards => {
                if let Some(ref video) = self.video {
                    let ts = video.timestamp();
                    if video.is_ended() {
                        video.seek(video.duration() - TEN_SECS_IN_MICROS);
                        video.play();
                        video.pause();
                        self.play_button.set_icon_name("play-symbolic");
                        sender.input(ViewOneInput::PlayToggle);
                    } else if ts < TEN_SECS_IN_MICROS {
                        video.seek(0);
                    } else {
                        video.seek(ts - TEN_SECS_IN_MICROS);
                    }
                }
            },
            ViewOneInput::SkipForward => {
                if let Some(ref video) = self.video {
                    let mut ts = video.timestamp();
                    if ts + TEN_SECS_IN_MICROS >= video.duration() {
                        ts = video.duration();
                        video.stream_ended();
                    } else {
                        ts += TEN_SECS_IN_MICROS;
                    }
                    video.seek(ts);
                }
            },
            ViewOneInput::VideoEnded => {
                self.play_button.set_icon_name("arrow-circular-top-left-symbolic");
                self.skip_forward.set_sensitive(false);
            },
            ViewOneInput::VideoTimestamp => {
                if let Some(ref video) = self.video {
                    let current_ts = fotema_core::time::format_hhmmss(&TimeDelta::microseconds(video.timestamp()));
                    let total_ts = fotema_core::time::format_hhmmss(&TimeDelta::microseconds(video.duration()));
                    self.video_timestamp.set_text(&format!("{}/{}", current_ts, total_ts));
                }
            },
            ViewOneInput::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                self.transcode_button.set_visible(false);
                let _ = sender.output(ViewOneOutput::TranscodeAll);
            },
            ViewOneInput::Refresh => {
                self.face_thumbnails.emit(FaceThumbnailsInput::Refresh);
            },
        }
    }
}

impl ViewOne {
    fn is_video_controls_visible(&self) -> bool {
        self.video.is_some() && !self.is_transcode_required
    }
}
