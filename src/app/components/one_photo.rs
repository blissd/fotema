// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use relm4::gtk;
use relm4::adw::gdk;
use relm4::gtk::gio;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;
use glycin;

use crate::app::components::photo_info::PhotoInfo;
use crate::app::components::photo_info::PhotoInfoInput;
use crate::app::components::progress_monitor::ProgressMonitor;
use crate::app::components::progress_panel::ProgressPanel;
use crate::app::SharedState;

use std::sync::Arc;

use tracing::{event, Level};

#[derive(Debug)]
pub enum OnePhotoInput {
    ViewPhoto(VisualId),

    ToggleInfo,

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,

    // Transcode all incompatible videos
    TranscodeAll,
}

#[derive(Debug)]
pub enum OnePhotoOutput {
    TranscodeAll,
}

pub struct OnePhoto {
    state: SharedState,

    // Photo to show
    picture: gtk::Picture,

    transcode_button: gtk::Button,

    transcode_status: adw::StatusPage,

    transcode_progress: Controller<ProgressPanel>,

    // Info for photo
    photo_info: Controller<PhotoInfo>,

    // Photo and photo info views
    split_view: adw::OverlaySplitView,

    // Window title, which should be the image/video name.
    title: String,

    // Visual ID of currently displayed item
    visual_id: Option<VisualId>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for OnePhoto {
    type Init = (SharedState, Arc<Reducer<ProgressMonitor>>);
    type Input = OnePhotoInput;
    type Output = OnePhotoOutput;

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                #[wrap(Some)]
                set_title_widget = &gtk::Label {
                    #[watch]
                    set_label: model.title.as_ref(),
                    add_css_class: "title",
                },
                pack_end = &gtk::Button {
                    set_icon_name: "info-outline-symbolic",
                    connect_clicked => OnePhotoInput::ToggleInfo,
                }
            },

            #[wrap(Some)]
            #[local_ref]
            set_content = &split_view -> adw::OverlaySplitView {
                set_collapsed: true,

                #[wrap(Some)]
                set_sidebar = model.photo_info.widget(),

                set_sidebar_position: gtk::PackType::End,

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

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
        }
    }

    async fn init(
        (state, transcode_progress_monitor): Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let picture = gtk::Picture::new();

        let split_view = adw::OverlaySplitView::new();

        let transcode_button = gtk::Button::new();

        let transcode_progress = ProgressPanel::builder()
            .launch(transcode_progress_monitor.clone())
            .detach();

        let transcode_status = adw::StatusPage::new();

        let photo_info = PhotoInfo::builder()
            .launch(state.clone())
            .detach();

        let model = OnePhoto {
            state,
            picture: picture.clone(),
            transcode_button: transcode_button.clone(),
            transcode_status: transcode_status.clone(),
            transcode_progress,
            photo_info,
            split_view: split_view.clone(),
            title: String::from("-"),
            visual_id: None,
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            OnePhotoInput::Hidden => {
                self.picture.set_paintable(None::<&gdk::Paintable>);
                self.title = String::from("-");
            },
            OnePhotoInput::ViewPhoto(visual_id) => {
                event!(Level::INFO, "Showing item for {}", visual_id);
                self.visual_id = None;

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

                self.title = visual_path.file_name()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or(String::from("-"));

                self.picture.set_paintable(None::<&gdk::Paintable>);

                if visual.is_photo_only() {
                    self.picture.set_visible(true);
                    self.transcode_status.set_visible(false);

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
                    self.photo_info.emit(PhotoInfoInput::Photo(visual_id.clone(), image.info().clone()));
                } else { // video or motion photo
                    self.photo_info.emit(PhotoInfoInput::Video(visual_id.clone()));

                    let is_transcoded = visual.video_transcoded_path.as_ref().is_some_and(|x| x.exists());

                    if visual.is_transcode_required.is_some_and(|x| x) && !is_transcoded {
                        self.picture.set_visible(false);
                        self.transcode_status.set_visible(true);
                        self.split_view.set_collapsed(true);
                    } else {
                        self.picture.set_visible(true);
                        self.transcode_status.set_visible(false);

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
                    }
                }
                self.visual_id = Some(visual_id);
            },
            OnePhotoInput::ToggleInfo => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_show_sidebar(!show);
            },
            OnePhotoInput::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                self.transcode_button.set_visible(false);
                let _ = sender.output(OnePhotoOutput::TranscodeAll);
            }
        }
    }
}
