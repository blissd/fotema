// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::TimeDelta;
use fotema_core::Visual;
use fotema_core::VisualId;
use fotema_core::visual::model::PictureOrientation;
use fotema_core::FlatpakPathBuf;

use glycin;
use relm4::adw::gdk;
use relm4::gtk;
use relm4::gtk::gio;
use relm4::gtk::prelude::*;
use relm4::prelude::*;
use relm4::*;
use strum::IntoEnumIterator;

use crate::app::components::progress_monitor::ProgressMonitor;
use crate::app::components::progress_panel::ProgressPanel;
use crate::fl;

use std::sync::Arc;

use tracing::{Level, debug, event, info};

const TEN_SECS_IN_MICROS: i64 = 10_000_000;
const FIFTEEN_SECS_IN_MICROS: i64 = 15_000_000;

#[derive(Debug, Eq, PartialEq)]
pub enum Viewing {
    Photo,
    MotionPhoto,
    Video,
    Transcode,
    Error,
    None,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Audio {
    Muted,
    Audible,
    None,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Playback {
    Playing,
    Paused,
    Ended,
    None,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Broken {
    /// Visual item no longer on file system.
    MissingInFileSystem(FlatpakPathBuf),

    /// Glycin couldn't load the file.
    Failed,

    /// Not broken.
    None,
}

#[derive(Debug)]
pub enum ViewOneInput {
    // Load an item.
    Load(Arc<Visual>),

    // View an item.
    View,

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,

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
    /// User has clicked transcode button.
    TranscodeAll,

    /// Successfully showing a photo.
    PhotoShown(VisualId, glycin::ImageDetails),

    /// Successfully showing a video.
    VideoShown(VisualId),

    /// Error showing photo or video.
    ErrorShown(VisualId),

    /// Showing transcode status.
    TranscodeShown(VisualId),
    // TODO is a NothingShown value needed?
}

pub struct ViewOne {
    viewing: Viewing,
    audio: Audio,
    playback: Playback,
    broken: Broken,

    picture: gtk::Picture,

    video: Option<gtk::MediaFile>,

    /// Info for loaded image
    image_info: Option<glycin::ImageDetails>,

    visual_id: Option<VisualId>,

    /// Should the video skip backwards/forwards buttons be enabled.
    is_skipping_allowed: bool,

    /// Label text displaying video timestamp
    video_timestamp: String,

    transcode_progress: Controller<ProgressPanel>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for ViewOne {
    type Init = Arc<Reducer<ProgressMonitor>>;
    type Input = ViewOneInput;
    type Output = ViewOneOutput;

    view! {
        #[root]
        gtk::Overlay {
            set_vexpand: true,
            set_hexpand: true,

            // video_controls
            add_overlay = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 12,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::End,

                #[watch]
                set_visible: model.viewing == Viewing::Video || model.viewing == Viewing::MotionPhoto,

                gtk::Frame {
                    set_halign: gtk::Align::Center,
                    add_css_class: "osd",

                    #[watch]
                    set_visible: model.viewing == Viewing::Video,

                    #[wrap(Some)]
                    set_child = &gtk::Label {
                        set_halign: gtk::Align::Center,
                        add_css_class: "photo-grid-month-label",

                        #[watch]
                        set_text: &model.video_timestamp,
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

                    #[watch]
                    set_visible: model.viewing == Viewing::Video || model.viewing == Viewing::MotionPhoto,

                    gtk::Button {
                        set_icon_name: "skip-backwards-10-symbolic",
                        add_css_class: "circular",
                        add_css_class: "osd",
                        set_tooltip_text: Some(&fl!("viewer-skip-backwards-10-seconds", "tooltip")),

                        #[watch]
                        set_visible: model.viewing == Viewing::Video && model.is_skipping_allowed,

                        #[watch]
                        set_sensitive: model.playback == Playback::Playing
                            && model.is_skipping_allowed,

                        connect_clicked => ViewOneInput::SkipBackwards,
                    },

                    gtk::Button {
                        #[watch]
                        set_icon_name: model.play_button_icon_name(),

                        add_css_class: "circular",
                        add_css_class: "osd",
                        set_tooltip_text: Some(&fl!("viewer-play", "tooltip")),

                        #[watch]
                        set_visible: model.viewing == Viewing::Video || model.viewing == Viewing::MotionPhoto,

                        connect_clicked => ViewOneInput::PlayToggle,
                    },

                    gtk::Button {
                        set_icon_name: "skip-forward-10-symbolic",
                        add_css_class: "circular",
                        add_css_class: "osd",
                        set_tooltip_text: Some(&fl!("viewer-skip-forward-10-seconds", "tooltip")),

                        #[watch]
                        set_visible: model.viewing == Viewing::Video && model.is_skipping_allowed,

                        #[watch]
                        set_sensitive: model.playback == Playback::Playing
                            && model.is_skipping_allowed,

                        connect_clicked => ViewOneInput::SkipForward,
                    },

                    gtk::Button {
                        #[watch]
                        set_icon_name: model.mute_button_icon_name(),

                        set_margin_start: 36,
                        add_css_class: "circular",
                        add_css_class: "osd",
                        set_tooltip_text: Some(&fl!("viewer-mute", "tooltip")),

                        #[watch]
                        set_visible: model.viewing == Viewing::Video || model.viewing == Viewing::MotionPhoto,

                        connect_clicked => ViewOneInput::MuteToggle,
                    }
                }
            },

            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                gtk::Box {
                    set_vexpand: true,
                    set_halign: gtk::Align::Center,

                    #[watch]
                    set_visible: model.viewing == Viewing::Photo || model.viewing == Viewing::MotionPhoto || model.viewing == Viewing::Video,

                    #[local_ref]
                    picture -> gtk::Picture {}
                },

                adw::StatusPage {
                    set_valign: gtk::Align::Start,
                    set_vexpand: true,

                    set_icon_name: Some("playback-error-symbolic"),
                    set_description: Some(&fl!("viewer-convert-all-description")),

                    #[watch]
                    set_visible: model.viewing == Viewing::Transcode,

                    #[wrap(Some)]
                    set_child = &adw::Clamp {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_maximum_size: 400,

                        #[wrap(Some)]
                        set_child = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            // FIXME hide while transcodes are in progress
                            gtk::Button {
                                set_label: &fl!("viewer-convert-all-button"),
                                add_css_class: "suggested-action",
                                add_css_class: "pill",
                                connect_clicked => ViewOneInput::TranscodeAll,
                            },

                            model.transcode_progress.widget(),
                        }
                    }
                },

                adw::StatusPage {
                    set_valign: gtk::Align::Start,
                    set_vexpand: true,

                    #[watch]
                    set_icon_name: model.broken_status_icon_name(),

                    #[watch]
                    set_description: model.broken_status_description().as_ref().map(|x| x.as_str()),

                    #[watch]
                    set_visible: model.viewing == Viewing::Error,
                }
            }
        }
    }

    async fn init(
        transcode_progress_monitor: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let picture = gtk::Picture::new();

        let transcode_progress = ProgressPanel::builder()
            .launch(transcode_progress_monitor.clone())
            .detach();

        let model = ViewOne {
            viewing: Viewing::None,
            audio: Audio::None,
            playback: Playback::None,
            broken: Broken::None,

            picture: picture.clone(),
            video: None,
            image_info: None,
            visual_id: None,
            is_skipping_allowed: false,
            video_timestamp: "".into(),
            transcode_progress,
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            ViewOneInput::Load(visual) => {
                info!("Load visual {}", visual.visual_id);

                let visual_sandbox_path = visual.sandbox_path();

                self.viewing = Viewing::None;
                self.audio = Audio::None;
                self.playback = Playback::None;
                self.broken = Broken::None;
                self.is_skipping_allowed = false;
                self.visual_id = None;

                if !visual_sandbox_path.exists() {
                    self.viewing = Viewing::Error;
                    self.broken = Broken::MissingInFileSystem(visual.path().clone());
                    return;
                }

                self.picture.set_paintable(None::<&gdk::Paintable>);
                self.video = None;
                self.image_info = None;

                self.visual_id = Some(visual.visual_id.clone());

                // clear orientation transformation css classes
                for orient in PictureOrientation::iter() {
                    self.picture.remove_css_class(orient.as_ref());
                }

                if visual.is_photo_only() {
                    self.viewing = Viewing::Photo;

                    // Apply a CSS transformation to respect the EXIF orientation
                    // NOTE: don't use Glycin to apply the transformation here because it is
                    // too slow.
                    let orientation = visual
                        .picture_orientation
                        .unwrap_or(PictureOrientation::North);
                    self.picture.add_css_class(orientation.as_ref());

                    let file = gio::File::for_path(visual_sandbox_path);

                    let mut loader = glycin::Loader::new(file);
                    loader.apply_transformations(false);

                    let image = loader.load().await;

                    let Ok(image) = image else {
                        event!(Level::ERROR, "Failed loading image: {:?}", image);
                        self.viewing = Viewing::Error;
                        self.broken = Broken::Failed;
                        return;
                    };

                    let frame = image.next_frame().await;
                    let Ok(frame) = frame else {
                        event!(Level::ERROR, "Failed getting image frame: {:?}", frame);
                        self.viewing = Viewing::Error;
                        self.broken = Broken::Failed;
                        return;
                    };

                    self.image_info = Some(image.details().clone());

                    let texture = frame.texture();
                    self.picture.set_paintable(Some(&texture));
                } else {
                    // video or motion photo
                    let is_transcoded = visual
                        .video_transcoded_path
                        .as_ref()
                        .is_some_and(|x| x.exists());
                    let is_transcode_required =
                        visual.is_transcode_required.is_some_and(|x| x) && !is_transcoded;

                    if is_transcode_required {
                        self.viewing = Viewing::Transcode;
                    } else {
                        // if a video is transcoded then the rotation transformation will
                        // already have been applied.
                        if !is_transcoded {
                            // Apply a CSS transformation to respect the display matrix rotation
                            let orientation = visual
                                .video_orientation
                                .unwrap_or(PictureOrientation::North);
                            self.picture.add_css_class(orientation.as_ref());
                        }

                        let video_path = visual
                            .video_transcoded_path.clone()
                            .filter(|x| x.exists())
                            .or_else(|| visual.video_path.clone().map(|p| p.sandbox_path))
                            .filter(|x| x.exists())
                            .or_else(|| visual.motion_photo_video_path.clone())
                            .expect("must have video path");

                        debug!("Video path is: {:?}", video_path);

                        let video = gtk::MediaFile::for_filename(video_path);
                        if visual.is_motion_photo() {
                            debug!("Is a motion photo");
                            self.viewing = Viewing::MotionPhoto;

                            self.playback = Playback::Playing;
                            video.set_loop(true);

                            self.audio = Audio::Muted;
                            video.set_muted(true);
                        } else {
                            self.viewing = Viewing::Video;

                            self.playback = Playback::Paused;
                            video.set_loop(false);

                            // Instead of video.set_muted(false), we must mute and then
                            // send a message to unmute. This seems to work around the problem
                            // of videos staying muted after viewing muting and unmuting.
                            self.audio = Audio::Muted;
                            video.set_muted(true);
                            sender.input(ViewOneInput::MuteToggle);

                            let sender1 = sender.clone();
                            let sender2 = sender.clone();
                            let sender3 = sender.clone();
                            video.connect_ended_notify(move |_| {
                                sender1.input(ViewOneInput::VideoEnded)
                            });
                            video.connect_timestamp_notify(move |_| {
                                sender2.input(ViewOneInput::VideoTimestamp)
                            });
                            video.connect_prepared_notify(move |_| {
                                sender3.input(ViewOneInput::VideoPrepared)
                            });
                        }

                        self.video = Some(video);
                        self.picture.set_paintable(self.video.as_ref());
                    }
                }
            }
            ViewOneInput::View => {
                info!("View");

                let Some(visual_id) = self.visual_id.as_ref() else {
                    return;
                };

                match self.viewing {
                    Viewing::Photo => {
                        let Some(info) = self.image_info.as_ref() else {
                            return;
                        };
                        let _ = sender
                            .output(ViewOneOutput::PhotoShown(visual_id.clone(), info.clone()));
                    }
                    Viewing::MotionPhoto | Viewing::Video => {
                        if let Some(video) = self.video.as_ref() {
                            debug!("Playing video");
                            self.playback = Playback::Playing;
                            video.play();
                        }
                        let _ = sender.output(ViewOneOutput::VideoShown(visual_id.clone()));
                    }
                    Viewing::Transcode => {
                        let _ = sender.output(ViewOneOutput::TranscodeShown(visual_id.clone()));
                    }
                    Viewing::Error => {
                        let _ = sender.output(ViewOneOutput::ErrorShown(visual_id.clone()));
                    }
                    Viewing::None => {}
                };
            }
            ViewOneInput::Hidden => {
                info!("Hide");
                if let Some(video) = self.video.as_ref() {
                    debug!("Pausing video");
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
                        self.playback = Playback::Paused;
                        sender.input(ViewOneInput::PlayToggle);
                    } else if video.is_playing() {
                        self.playback = Playback::Paused;
                        video.pause();
                    }
                }
            }
            ViewOneInput::VideoPrepared => {
                // Video details, like duration, aren't available until the video
                // has been prepared.
                if let Some(ref video) = self.video {
                    // Only enable the skip buttons if the video is long enough for
                    // skipping in chunks of 10 seconds to make some sense.
                    self.is_skipping_allowed = video.duration() >= FIFTEEN_SECS_IN_MICROS;
                }
            }
            ViewOneInput::MuteToggle => {
                if let Some(ref video) = self.video {
                    if video.is_muted() {
                        self.audio = Audio::Audible;
                        video.set_muted(false);
                    } else {
                        self.audio = Audio::Muted;
                        video.set_muted(true);
                    }
                }
            }
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
                        self.playback = Playback::Ended;
                        sender.input(ViewOneInput::PlayToggle);
                    } else if video.is_playing() {
                        self.playback = Playback::Paused;
                        video.pause();
                    } else {
                        // is paused
                        self.playback = Playback::Playing;
                        video.play();
                    }
                }
            }
            ViewOneInput::SkipBackwards => {
                if let Some(ref video) = self.video {
                    let ts = video.timestamp();
                    if video.is_ended() {
                        video.seek(video.duration() - TEN_SECS_IN_MICROS);
                        video.play();
                        video.pause();
                        self.playback = Playback::Ended;
                        sender.input(ViewOneInput::PlayToggle);
                    } else if ts < TEN_SECS_IN_MICROS {
                        video.seek(0);
                    } else {
                        video.seek(ts - TEN_SECS_IN_MICROS);
                    }
                }
            }
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
            }
            ViewOneInput::VideoEnded => {
                self.playback = Playback::Ended;
            }
            ViewOneInput::VideoTimestamp => {
                if let Some(ref video) = self.video {
                    let current_ts = fotema_core::time::format_hhmmss(&TimeDelta::microseconds(
                        video.timestamp(),
                    ));
                    let total_ts = fotema_core::time::format_hhmmss(&TimeDelta::microseconds(
                        video.duration(),
                    ));
                    self.video_timestamp = format!("{}/{}", current_ts, total_ts).into();
                }
            }
            ViewOneInput::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                let _ = sender.output(ViewOneOutput::TranscodeAll);
            }
        }
    }
}

impl ViewOne {
    fn play_button_icon_name(&self) -> &str {
        match self.playback {
            Playback::Playing => "pause-symbolic",
            Playback::Ended => "arrow-circular-top-left-symbolic",
            Playback::Paused => "play-symbolic",
            Playback::None => "arrow-circular-top-left-symbolic",
        }
    }

    fn mute_button_icon_name(&self) -> &str {
        match self.audio {
            Audio::Audible => "multimedia-volume-control-symbolic",
            Audio::Muted => "audio-volume-muted-symbolic",
            Audio::None => "arrow-circular-top-left-symbolic",
        }
    }

    fn broken_status_icon_name(&self) -> Option<&str> {
        match self.broken {
            Broken::MissingInFileSystem(_) => Some("item-missing-symbolic"),
            Broken::Failed => Some("sad-computer-symbolic"),
            Broken::None => None,
        }
    }

    fn broken_status_description(&self) -> Option<String> {
        match self.broken {
            Broken::MissingInFileSystem(ref visual_path) => Some(fl!(
                "viewer-error-missing-file",
                file_name = visual_path.host_path.to_string_lossy()
            )),
            Broken::Failed => Some(fl!("viewer-error-failed-to-load")),
            Broken::None => None::<String>,
        }
    }
}
