// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;

use super::albums::album_filter::AlbumFilter;
use super::one_photo::{OnePhoto, OnePhotoInput, OnePhotoOutput};
use super::photo_info::{PhotoInfo, PhotoInfoInput};
use super::progress_monitor::ProgressMonitor;
use crate::app::SharedState;
use crate::adaptive;
use crate::fl;

use fotema_core::Visual;

use std::sync::Arc;

use tracing::{event, Level};

#[derive(Debug)]
pub enum ViewerInput {
    // View an item after applying an album filter.
    View(VisualId, AlbumFilter),

    ViewByIndex(usize),

    ToggleInfo,

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,

    ShowPhotoInfo(VisualId, glycin::ImageInfo),

    ShowVideoInfo(VisualId),

    // Transcode all incompatible videos
    TranscodeAll,

    // Go to the previous photo
    GoLeft,

    // Go to the next photo
    GoRight,

    // Adapt to layout
    Adapt(adaptive::Layout),
}

#[derive(Debug)]
pub enum ViewerOutput {
    TranscodeAll,
}

pub struct Viewer {
    state: SharedState,

    // Info for photo
    one_photo: AsyncController<OnePhoto>,

    // Info for photo
    photo_info: Controller<PhotoInfo>,

    // Photo and photo info views
    split_view: adw::OverlaySplitView,

    left_button: gtk::Button,
    right_button: gtk::Button,

    current_index: Option<usize>,

    // Album currently displayed item is a member of
    filter: AlbumFilter,

    // Visual items filtered by album filter.
    // This is to support the next and previous buttons.
    filtered_items: Vec<Arc<Visual>>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for Viewer {
    type Init = (SharedState, Arc<Reducer<ProgressMonitor>>, Arc<adaptive::LayoutState>);
    type Input = ViewerInput;
    type Output = ViewerOutput;

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                pack_end = &gtk::Button {
                    set_icon_name: "info-outline-symbolic",
                    set_tooltip_text: Some(&fl!("viewer-info-tooltip")),
                    connect_clicked => ViewerInput::ToggleInfo,
                }
            },

            #[wrap(Some)]
            #[local_ref]
            set_content = &split_view -> adw::OverlaySplitView {
                set_collapsed: false,
                set_show_sidebar: false,

                #[wrap(Some)]
                set_sidebar = model.photo_info.widget(),

                set_sidebar_position: gtk::PackType::End,

                #[wrap(Some)]
                set_content = &gtk::Overlay {
                    add_overlay =  &gtk::Box {
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Center,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_all: 18,
                        set_spacing: 12,

                        #[local_ref]
                        left_button -> gtk::Button {
                            set_icon_name: "left-symbolic",
                            add_css_class: "osd",
                            add_css_class: "circular",
                            set_tooltip_text: Some(&fl!("viewer-previous", "tooltip")),
                            connect_clicked => ViewerInput::GoLeft,
                        },
                    },

                    add_overlay =  &gtk::Box {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Center,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_all: 18,
                        set_spacing: 12,

                        #[local_ref]
                        right_button -> gtk::Button {
                            set_icon_name: "right-symbolic",
                            add_css_class: "osd",
                            add_css_class: "circular",
                            set_tooltip_text: Some(&fl!("viewer-next", "tooltip")),
                            connect_clicked => ViewerInput::GoRight,
                        },
                    },

                    #[wrap(Some)]
                    set_child = model.one_photo.widget(),
                },
            }
        }
    }

    async fn init(
        (state, transcode_progress_monitor, layout_state): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let split_view = adw::OverlaySplitView::new();

        let one_photo = OnePhoto::builder()
            .launch(transcode_progress_monitor)
            .forward(sender.input_sender(), |msg| match msg {
                OnePhotoOutput::PhotoShown(id, info) => ViewerInput::ShowPhotoInfo(id, info),
                OnePhotoOutput::VideoShown(id) => ViewerInput::ShowVideoInfo(id),
                OnePhotoOutput::TranscodeAll => ViewerInput::TranscodeAll,
            });

        let photo_info = PhotoInfo::builder()
            .launch(state.clone())
            .detach();

        layout_state.subscribe(sender.input_sender(), |layout| ViewerInput::Adapt(layout.clone()));

        let left_button = gtk::Button::new();
        let right_button = gtk::Button::new();

        let model = Viewer {
            state,
            one_photo,
            photo_info,
            current_index: None,
            left_button: left_button.clone(),
            right_button: right_button.clone(),
            split_view: split_view.clone(),
            filter: AlbumFilter::None,
            filtered_items: Vec::new(),
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            ViewerInput::Hidden => {
                self.one_photo.emit(OnePhotoInput::Hidden);
            },
            ViewerInput::View(visual_id, filter) => {
                event!(Level::INFO, "Showing item for {}", visual_id);

                // To support next/previous navigation we must have a view of the visual
                // items filtered with the same album filter as the album the user is currently
                // looking at.
               if self.filter != filter {
                    self.filter = filter.clone();
                    let items = self.state.read();
                    self.filtered_items = items.iter()
                        .filter(|v| filter.clone().filter(&v))
                        .map(|v| v.clone())
                        .collect();
                }

                self.current_index = self.filtered_items
                    .iter()
                    .position(|x| x.visual_id == visual_id);

                if let Some(index) = self.current_index {
                    sender.input(ViewerInput::ViewByIndex(index));
                }
            },
            ViewerInput::ViewByIndex(index) => {

                if index >= self.filtered_items.len() || self.filtered_items.is_empty() {
                    event!(Level::ERROR, "Cannot view at index {}. Number of filtered_items is {}", index, self.filtered_items.len());
                    return;
                }

                let visual = &self.filtered_items[index];
                self.current_index = Some(index);

                self.update_nav_buttons();

                self.one_photo.emit(OnePhotoInput::View(visual.clone()));
            },
            ViewerInput::ToggleInfo => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_show_sidebar(!show);
            },
            ViewerInput::ShowPhotoInfo(visual_id, image_info) => {
                self.photo_info.emit(PhotoInfoInput::Photo(visual_id, image_info));
            },
            ViewerInput::ShowVideoInfo(visual_id) => {
                self.photo_info.emit(PhotoInfoInput::Video(visual_id));
            },
            ViewerInput::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                // FIXME refactor to remove message forwarding.
                // OnePhoto should send straight to transcoder.
                let _ = sender.output(ViewerOutput::TranscodeAll);
            },
            ViewerInput::GoLeft => {
                let Some(index) = self.current_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                sender.input(ViewerInput::ViewByIndex(index - 1));
            },
            ViewerInput::GoRight => {
                let Some(index) = self.current_index else {
                    return;
                };

                if index + 1 >= self.filtered_items.len() {
                    return;
                }

                sender.input(ViewerInput::ViewByIndex(index + 1));
            },
            ViewerInput::Adapt(adaptive::Layout::Narrow) => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_collapsed(true);
                self.split_view.set_show_sidebar(show);
            },
            ViewerInput::Adapt(adaptive::Layout::Wide) => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_collapsed(false);
                self.split_view.set_show_sidebar(show);
            },
        }
    }
}

impl Viewer {
    fn update_nav_buttons(&self) {
        if self.filtered_items.is_empty() {
            self.left_button.set_sensitive(false);
            self.right_button.set_sensitive(false);
        }

        let Some(index) = self.current_index else {
            return;
        };

        if index == 0 {
            self.left_button.set_sensitive(false);
            self.right_button.set_sensitive(true);
        } else if index == self.filtered_items.len() -1 {
            self.left_button.set_sensitive(true);
            self.right_button.set_sensitive(false);
        } else {
            self.left_button.set_sensitive(true);
            self.right_button.set_sensitive(true);
        }
    }
}
