// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;

use crate::app::components::albums::album_filter::AlbumFilter;
use super::view_one::{ViewOne, ViewOneInput, ViewOneOutput};
use super::view_info::{ViewInfo, ViewInfoInput};
use crate::app::components::progress_monitor::ProgressMonitor;
use crate::app::SharedState;
use crate::adaptive;
use crate::fl;

use fotema_core::Visual;

use std::sync::Arc;

use tracing::{event, Level};

#[derive(Debug)]
pub enum ViewNavInput {
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
pub enum ViewNavOutput {
    TranscodeAll,
}

pub struct ViewNav {
    state: SharedState,

    // View one photo or video
    view_one: AsyncController<ViewOne>,

    // Info for photo
    view_info: Controller<ViewInfo>,

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
impl SimpleAsyncComponent for ViewNav {
    type Init = (SharedState, Arc<Reducer<ProgressMonitor>>, Arc<adaptive::LayoutState>);
    type Input = ViewNavInput;
    type Output = ViewNavOutput;

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                pack_end = &gtk::Button {
                    set_icon_name: "info-outline-symbolic",
                    set_tooltip_text: Some(&fl!("viewer-info-tooltip")),
                    connect_clicked => ViewNavInput::ToggleInfo,
                }
            },

            #[wrap(Some)]
            #[local_ref]
            set_content = &split_view -> adw::OverlaySplitView {
                set_collapsed: false,
                set_show_sidebar: false,

                #[wrap(Some)]
                set_sidebar = model.view_info.widget(),

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
                            connect_clicked => ViewNavInput::GoLeft,
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
                            connect_clicked => ViewNavInput::GoRight,
                        },
                    },

                    #[wrap(Some)]
                    set_child = model.view_one.widget(),
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

        let view_one = ViewOne::builder()
            .launch(transcode_progress_monitor)
            .forward(sender.input_sender(), |msg| match msg {
                ViewOneOutput::PhotoShown(id, info) => ViewNavInput::ShowPhotoInfo(id, info),
                ViewOneOutput::VideoShown(id) => ViewNavInput::ShowVideoInfo(id),
                ViewOneOutput::TranscodeAll => ViewNavInput::TranscodeAll,
            });

        let view_info = ViewInfo::builder()
            .launch(state.clone())
            .detach();

        layout_state.subscribe(sender.input_sender(), |layout| ViewNavInput::Adapt(*layout));

        let left_button = gtk::Button::new();
        let right_button = gtk::Button::new();

        let model = ViewNav {
            state,
            view_one,
            view_info,
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
            ViewNavInput::Hidden => {
                self.view_one.emit(ViewOneInput::Hidden);
            },
            ViewNavInput::View(visual_id, filter) => {
                event!(Level::INFO, "Showing item for {}", visual_id);

                // To support next/previous navigation we must have a view of the visual
                // items filtered with the same album filter as the album the user is currently
                // looking at.
               if self.filter != filter {
                    self.filter = filter.clone();
                    let items = self.state.read();
                    self.filtered_items = items.iter()
                        .filter(|v| filter.clone().filter(v))
                        .cloned()
                        .collect();
                }

                self.current_index = self.filtered_items
                    .iter()
                    .position(|x| x.visual_id == visual_id);

                if let Some(index) = self.current_index {
                    sender.input(ViewNavInput::ViewByIndex(index));
                }
            },
            ViewNavInput::ViewByIndex(index) => {

                if index >= self.filtered_items.len() || self.filtered_items.is_empty() {
                    event!(Level::ERROR, "Cannot view at index {}. Number of filtered_items is {}", index, self.filtered_items.len());
                    return;
                }

                let visual = &self.filtered_items[index];
                self.current_index = Some(index);

                self.update_nav_buttons();

                self.view_one.emit(ViewOneInput::View(visual.clone()));
            },
            ViewNavInput::ToggleInfo => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_show_sidebar(!show);
            },
            ViewNavInput::ShowPhotoInfo(visual_id, image_info) => {
                self.view_info.emit(ViewInfoInput::Photo(visual_id, image_info));
            },
            ViewNavInput::ShowVideoInfo(visual_id) => {
                self.view_info.emit(ViewInfoInput::Video(visual_id));
            },
            ViewNavInput::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                // FIXME refactor to remove message forwarding.
                // ViewOne should send straight to transcoder.
                let _ = sender.output(ViewNavOutput::TranscodeAll);
            },
            ViewNavInput::GoLeft => {
                let Some(index) = self.current_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                sender.input(ViewNavInput::ViewByIndex(index - 1));
            },
            ViewNavInput::GoRight => {
                let Some(index) = self.current_index else {
                    return;
                };

                if index + 1 >= self.filtered_items.len() {
                    return;
                }

                sender.input(ViewNavInput::ViewByIndex(index + 1));
            },
            ViewNavInput::Adapt(adaptive::Layout::Narrow) => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_collapsed(true);
                self.split_view.set_show_sidebar(show);
            },
            ViewNavInput::Adapt(adaptive::Layout::Wide) => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_collapsed(false);
                self.split_view.set_show_sidebar(show);
            },
        }
    }
}

impl ViewNav {
    fn update_nav_buttons(&self) {
        if self.filtered_items.len() <= 1 {
            self.left_button.set_sensitive(false);
            self.right_button.set_sensitive(false);
            return;
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
