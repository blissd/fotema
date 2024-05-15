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

use crate::app::components::album_filter::AlbumFilter;
use crate::app::components::photo_info::PhotoInfo;
use crate::app::components::photo_info::PhotoInfoInput;
use crate::app::components::progress_monitor::ProgressMonitor;
use crate::app::components::progress_panel::ProgressPanel;
use crate::app::SharedState;
use fotema_core::Visual;

use std::sync::Arc;

use tracing::{event, Level};

#[derive(Debug)]
pub enum OnePhotoInput {
    // View an item after applying an album filter.
    View(VisualId, AlbumFilter),

    ToggleInfo,

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,

    // Transcode all incompatible videos
    TranscodeAll,

    // Go to the previous photo
    GoLeft,

    // Go to the next photo
    GoRight,
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

    // Album currently displayed item is a member of
    filter: AlbumFilter,

    // Visual items filtered by album filter.
    // This is to support the next and previous buttons.
    filtered_items: Vec<Arc<Visual>>,
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
                set_collapsed: false,

                #[wrap(Some)]
                set_sidebar = model.photo_info.widget(),

                set_sidebar_position: gtk::PackType::End,

                #[wrap(Some)]
                set_content = &gtk::Overlay {
                    add_overlay =  &gtk::Box {
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::End,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_all: 18,
                        set_spacing: 12,

                        gtk::Button {
                            set_icon_name: "left-symbolic",
                            add_css_class: "osd",
                            add_css_class: "circular",
                            connect_clicked => OnePhotoInput::GoLeft,
                        },
                        gtk::Button {
                            set_icon_name: "right-symbolic",
                            add_css_class: "osd",
                            add_css_class: "circular",
                            connect_clicked => OnePhotoInput::GoRight,
                        },
                    },

                    #[wrap(Some)]
                    set_child = &gtk::Box {
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
                },
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
            filter: AlbumFilter::None,
            filtered_items: Vec::new(),
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
            OnePhotoInput::View(visual_id, filter) => {
                event!(Level::INFO, "Showing item for {}", visual_id);
                self.visual_id = None;

                // To support next/previous navigation we must have a view of the visual
                // items filtered with the same album filter as the album the user is currently
                // looking at.
                if self.filter != filter {
                    println!("FILTERING");
                    self.filter = filter.clone();
                    let items = self.state.read();
                    self.filtered_items = items.iter()
                        .filter(|v| filter.clone().filter(&v))
                        .map(|v| v.clone())
                        .collect();
                }

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
            },
            OnePhotoInput::GoLeft => {
                let Some(ref visual_id) = self.visual_id else {
                    return;
                };

                let cur_index = self.filtered_items
                    .iter()
                    .position(|ref x| x.visual_id == *visual_id);

                let Some(cur_index) = cur_index else {
                    return;
                };

                if cur_index > 0 {
                    let visual_id = self.filtered_items[cur_index-1].visual_id.clone();
                    sender.input(OnePhotoInput::View(visual_id, self.filter.clone()));
                }
            },
            OnePhotoInput::GoRight => {
                let Some(ref visual_id) = self.visual_id else {
                    return;
                };

                let cur_index = self.filtered_items
                    .iter()
                    .position(|ref x| x.visual_id == *visual_id);

                let Some(cur_index) = cur_index else {
                    return;
                };

                if cur_index + 1 < self.filtered_items.len() {
                    let visual_id = self.filtered_items[cur_index + 1].visual_id.clone();
                    sender.input(OnePhotoInput::View(visual_id, self.filter.clone()));
                }
            },
        }
    }
}
