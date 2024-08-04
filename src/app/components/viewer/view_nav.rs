// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;
use relm4::actions::{RelmAction, RelmActionGroup};

use crate::app::components::albums::album_filter::AlbumFilter;
use super::view_one::{ViewOne, ViewOneInput, ViewOneOutput};
use super::view_info::{ViewInfo, ViewInfoInput};
use crate::app::components::progress_monitor::ProgressMonitor;
use crate::app::SharedState;
use crate::adaptive;
use crate::fl;

use fotema_core::Visual;
use fotema_core::people;
use fotema_core::PictureId;
use fotema_core::VisualId;

use std::sync::Arc;

use tracing::{error, info};

// FIXME does the faces menu definition and action handling belong here?
// Maybe it belongs in view_one.rs or in face_thumbnails.rs?
relm4::new_action_group!(ViewNavActionGroup, "viewnav");

// Restore all ignored faces.
relm4::new_stateless_action!(RestoreIgnoredFacesAction, ViewNavActionGroup, "restore_ignored_faces");

// Ignore all faces that aren't associated with a person.
relm4::new_stateless_action!(IgnoreUnknownFacesAction, ViewNavActionGroup, "ignore_unknown_faces");

// Scan file for faces again using the most thorough scan possible.
relm4::new_stateless_action!(ScanForFacesAction, ViewNavActionGroup, "scan_faces");

#[derive(Debug)]
pub enum ViewNavInput {
    /// View an item after applying an album filter.
    View(VisualId, AlbumFilter),

    /// View item by index in filtered shared state.
    ViewByIndex(usize),

    /// Show/hide info bar
    ToggleInfo,

    /// The photo/video page has been hidden so any playing media should stop.
    Hidden,

    /// Inform info bar of photo details.
    ShowPhotoInfo(VisualId, glycin::ImageInfo),

    /// Inform info bar of video details.
    ShowVideoInfo(VisualId),

    /// Transcode all incompatible videos
    TranscodeAll,

    /// Go to the previous photo
    GoLeft,

    /// Go to the next photo
    GoRight,

    /// Adapt to layout
    Adapt(adaptive::Layout),

    /// Restore ignored faces for item.
    RestoreIgnoredFaces,

    /// Ignore all unknown faces for item
    IgnoreUnknownFaces,

    /// Scan for more faces.
    ScanForFaces,
}

#[derive(Debug)]
pub enum ViewNavOutput {
    TranscodeAll,
    ScanForFaces(PictureId),
}

pub struct ViewNav {
    state: SharedState,

    people_repo: people::Repository,

    // View one photo or video
    view_one: AsyncController<ViewOne>,

    // Info for photo
    view_info: Controller<ViewInfo>,

    // Photo and photo info views
    split_view: adw::OverlaySplitView,

    left_button: gtk::Button,
    right_button: gtk::Button,

    /// Index into shared state for currently viewed item.
    current_index: Option<usize>,

    // Album currently displayed item is a member of
    filter: AlbumFilter,

    // Visual items filtered by album filter.
    // This is to support the next and previous buttons.
    filtered_items: Vec<Arc<Visual>>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for ViewNav {
    type Init = (SharedState, Arc<Reducer<ProgressMonitor>>, Arc<adaptive::LayoutState>, people::Repository);
    type Input = ViewNavInput;
    type Output = ViewNavOutput;

    menu! {
        viewnav_menu: {
            section! {
                &fl!("viewer-faces-menu", "restore-ignored") => RestoreIgnoredFacesAction,
                &fl!("viewer-faces-menu", "ignore-unknown") => IgnoreUnknownFacesAction,
                &fl!("viewer-faces-menu", "scan") => ScanForFacesAction,
            }
        }
    }

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                pack_end = &gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::MenuButton {
                        set_icon_name: "sentiment-very-satisfied-symbolic",
                        set_menu_model: Some(&viewnav_menu),
                    },

                    gtk::Button {
                        set_icon_name: "info-outline-symbolic",
                        set_tooltip_text: Some(&fl!("viewer-info-tooltip")),
                        connect_clicked => ViewNavInput::ToggleInfo,
                    },
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
        (state, transcode_progress_monitor, layout_state, people_repo): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let split_view = adw::OverlaySplitView::new();

        let view_one = ViewOne::builder()
            .launch((people_repo.clone(), transcode_progress_monitor))
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
            people_repo,
            view_one,
            view_info,
            current_index: None,
            left_button: left_button.clone(),
            right_button: right_button.clone(),
            split_view: split_view.clone(),
            filter: AlbumFilter::None,
            filtered_items: Vec::new(),
        };


        let restore_action = {
            let sender = sender.clone();
            RelmAction::<RestoreIgnoredFacesAction>::new_stateless(move |_| {
                let _ = sender.input(ViewNavInput::RestoreIgnoredFaces);
            })
        };

        let ignore_unknown_faces_action = {
            let sender = sender.clone();
            RelmAction::<IgnoreUnknownFacesAction>::new_stateless(move |_| {
                let _ = sender.input(ViewNavInput::IgnoreUnknownFaces);
            })
        };

        let scan_faces_action = {
            let sender = sender.clone();
            RelmAction::<ScanForFacesAction>::new_stateless(move |_| {
                let _ = sender.input(ViewNavInput::ScanForFaces);
            })
        };

        let mut actions = RelmActionGroup::<ViewNavActionGroup>::new();
        actions.add_action(restore_action);
        actions.add_action(ignore_unknown_faces_action);
        actions.add_action(scan_faces_action);
        actions.register_for_widget(&root);

        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            ViewNavInput::Hidden => {
                self.view_one.emit(ViewOneInput::Hidden);
            },
            ViewNavInput::View(visual_id, filter) => {
                info!("Showing item for {}", visual_id);

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
                    error!("Cannot view at index {}. Number of filtered_items is {}", index, self.filtered_items.len());
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
                info!("Transcode all");
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

                self.view_one.emit(ViewOneInput::Hidden);
                sender.input(ViewNavInput::ViewByIndex(index - 1));
            },
            ViewNavInput::GoRight => {
                let Some(index) = self.current_index else {
                    return;
                };

                if index + 1 >= self.filtered_items.len() {
                    return;
                }

                self.view_one.emit(ViewOneInput::Hidden);
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
            ViewNavInput::RestoreIgnoredFaces => {
                let Some(index) = self.current_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                info!("Restoring unknown faces for");

                let visual = &self.filtered_items[index];
                if let Some(picture_id) = visual.picture_id {
                    if let Err(e) = self.people_repo.restore_ignored_faces(picture_id) {
                        error!("Failed restoring ignored faces: {}", e);
                    }
                }

                self.view_one.emit(ViewOneInput::Refresh);
            },
            ViewNavInput::IgnoreUnknownFaces => {
                let Some(index) = self.current_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                info!("Ignoring unknown faces");

                let visual = &self.filtered_items[index];
                if let Some(picture_id) = visual.picture_id {
                    if let Err(e) = self.people_repo.ignore_unknown_faces(picture_id) {
                        error!("Failed ignoring unknown faces: {}", e);
                    }
                }

                self.view_one.emit(ViewOneInput::Refresh);
            },
            ViewNavInput::ScanForFaces => {
 let Some(index) = self.current_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                info!("Scan for more faces");

                let visual = &self.filtered_items[index];
                if let Some(picture_id) = visual.picture_id {
                    let _ = sender.output(ViewNavOutput::ScanForFaces(picture_id));
                }
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
