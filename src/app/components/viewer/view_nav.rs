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

use tracing::{debug, error, info};

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

    // Carousel has been swiped to a new page. u32 is page index (0..2).
    SwipeTo(u32),

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

    /// Carousel for swiping through items
    carousel: adw::Carousel,

    /// Three pages of items (left, middle, right) to support "infinite swiping".
    carousel_pages: Vec<AsyncController<ViewOne>>,

    /// Page index of previous action.
    carousel_last_page_index: u32,

    // Info for photo
    view_info: Controller<ViewInfo>,

    /// Index into shared state for currently viewed item.
    album_index: Option<usize>,

    // Album currently displayed item is a member of
    album_filter: AlbumFilter,

    // Visual items filtered by album filter.
    // This is to support the next and previous buttons.
    album: Vec<Arc<Visual>>,

    //
    is_narrow: bool,

    shows_infobar: bool,
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
            #[name(split_view)]
            set_content = &adw::OverlaySplitView {
                #[watch]
                set_collapsed: model.is_narrow,

                #[watch]
                set_show_sidebar: model.shows_infobar,

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


                        #[name(left_button)]
                        gtk::Button {
                            set_icon_name: "left-symbolic",
                            add_css_class: "osd",
                            add_css_class: "circular",
                            set_tooltip_text: Some(&fl!("viewer-previous", "tooltip")),

                            #[watch]
                            set_sensitive: model.is_left_button_sensitive(),

                            connect_clicked => ViewNavInput::GoLeft,
                        },
                    },

                    add_overlay =  &gtk::Box {
                        set_halign: gtk::Align::End,
                        set_valign: gtk::Align::Center,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_all: 18,
                        set_spacing: 12,

                        #[name(right_button)]
                        gtk::Button {
                            set_icon_name: "right-symbolic",
                            add_css_class: "osd",
                            add_css_class: "circular",
                            set_tooltip_text: Some(&fl!("viewer-next", "tooltip")),

                            #[watch]
                            set_sensitive: model.is_right_button_sensitive(),

                            connect_clicked => ViewNavInput::GoRight,
                        },
                    },

                    #[wrap(Some)]
                    #[local_ref]
                    set_child = &carousel -> adw::Carousel {
                        connect_page_changed => move |_, idx| {
                            sender.input(ViewNavInput::SwipeTo(idx));
                        },
                    }
                },
            }
        }
    }

    async fn init(
        (state, transcode_progress_monitor, layout_state, people_repo): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

       let mut carousel_pages = Vec::with_capacity(3);

        carousel_pages.push(ViewOne::builder()
            .launch((people_repo.clone(), transcode_progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ViewOneOutput::PhotoShown(id, info) => ViewNavInput::ShowPhotoInfo(id, info),
                ViewOneOutput::VideoShown(id) => ViewNavInput::ShowVideoInfo(id),
                ViewOneOutput::TranscodeAll => ViewNavInput::TranscodeAll,
            }));

        carousel_pages.push(ViewOne::builder()
            .launch((people_repo.clone(), transcode_progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ViewOneOutput::PhotoShown(id, info) => ViewNavInput::ShowPhotoInfo(id, info),
                ViewOneOutput::VideoShown(id) => ViewNavInput::ShowVideoInfo(id),
                ViewOneOutput::TranscodeAll => ViewNavInput::TranscodeAll,
            }));

        carousel_pages.push(ViewOne::builder()
            .launch((people_repo.clone(), transcode_progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ViewOneOutput::PhotoShown(id, info) => ViewNavInput::ShowPhotoInfo(id, info),
                ViewOneOutput::VideoShown(id) => ViewNavInput::ShowVideoInfo(id),
                ViewOneOutput::TranscodeAll => ViewNavInput::TranscodeAll,
            }));

        let carousel = adw::Carousel::builder()
            .build();

        let view_info = ViewInfo::builder()
            .launch(state.clone())
            .detach();

        layout_state.subscribe(sender.input_sender(), |layout| ViewNavInput::Adapt(*layout));

        let model = ViewNav {
            state,
            people_repo,
            carousel: carousel.clone(),
            carousel_pages,
            carousel_last_page_index: 0,
            view_info,
            album_index: None,
            album_filter: AlbumFilter::None,
            album: Vec::new(),
            is_narrow: false,
            shows_infobar: false,
        };

        let restore_action = {
            let sender = sender.clone();
            RelmAction::<RestoreIgnoredFacesAction>::new_stateless(move |_| {
                sender.input(ViewNavInput::RestoreIgnoredFaces);
            })
        };

        let ignore_unknown_faces_action = {
            let sender = sender.clone();
            RelmAction::<IgnoreUnknownFacesAction>::new_stateless(move |_| {
                sender.input(ViewNavInput::IgnoreUnknownFaces);
            })
        };

        let scan_faces_action = {
            let sender = sender.clone();
            RelmAction::<ScanForFacesAction>::new_stateless(move |_| {
                sender.input(ViewNavInput::ScanForFaces);
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
                self.album_index = None;
                self.carousel_pages.iter().for_each(|page| page.emit(ViewOneInput::Hidden));
            },
            ViewNavInput::View(visual_id, album_filter) => {
                info!("Showing item for {}", visual_id);
                while self.carousel.n_pages() > 0 {
                    self.carousel.remove(&self.carousel.nth_page(0));
                }

                // To support next/previous navigation we must have a view of the visual
                // items filtered with the same album filter as the album the user is currently
                // looking at.
               if self.album_filter != album_filter {
                    self.album_filter = album_filter.clone();
                    let items = self.state.read();
                    self.album = items.iter()
                        .filter(|v| album_filter.clone().filter(v))
                        .cloned()
                        .collect();
                }

                self.album_index = self.album
                    .iter()
                    .position(|x| x.visual_id == visual_id);

                let Some(index) = self.album_index else {
                    error!("Cannot find index for visual item {}", visual_id);
                    return;
                };

                // Carousel will be either one, two, or three pages depending
                // on how many items are in the album being viewed.
                if self.album.len() == 1 {
                    self.carousel.append(self.carousel_pages[0].widget());
                    self.carousel_pages[0].emit(ViewOneInput::View(self.album[0].clone()));
                    self.carousel_last_page_index = 0;
                } else if self.album.len() == 2 {
                    self.carousel.append(self.carousel_pages[0].widget());
                    self.carousel_pages[0].emit(ViewOneInput::View(self.album[0].clone()));
                    self.carousel_last_page_index = 0;

                    self.carousel.append(self.carousel_pages[1].widget());
                    self.carousel_pages[1].emit(ViewOneInput::View(self.album[1].clone()));

                    self.carousel.scroll_to(&self.carousel.nth_page(index as u32), false);
                } else if self.album.len() >= 3 {
                    self.carousel.append(self.carousel_pages[0].widget());
                    self.carousel.append(self.carousel_pages[1].widget());
                    self.carousel.append(self.carousel_pages[2].widget());

                    if index == 0 {
                        // Starting on _first_ item of album.
                        self.carousel_pages[0].emit(ViewOneInput::View(self.album[0].clone()));
                        self.carousel_pages[1].emit(ViewOneInput::View(self.album[1].clone()));
                        self.carousel_pages[2].emit(ViewOneInput::View(self.album[2].clone()));
                        self.carousel.scroll_to(&self.carousel.nth_page(0), false);
                        self.carousel_last_page_index = 0;
                    } else if index == self.album.len() - 1 {
                        // Starting on _last_ item of album.
                        self.carousel_pages[0].emit(ViewOneInput::View(self.album[index - 2].clone()));
                        self.carousel_pages[1].emit(ViewOneInput::View(self.album[index - 1].clone()));
                        self.carousel_pages[2].emit(ViewOneInput::View(self.album[index].clone()));
                        self.carousel.scroll_to(&self.carousel.nth_page(2), false);
                        self.carousel_last_page_index = 2;
                    } else {
                        // Starting somewhere between first and last item.
                        self.carousel_pages[0].emit(ViewOneInput::View(self.album[index - 1].clone()));
                        self.carousel_pages[1].emit(ViewOneInput::View(self.album[index].clone()));
                        self.carousel_pages[2].emit(ViewOneInput::View(self.album[index + 1].clone()));
                        self.carousel.scroll_to(&self.carousel.nth_page(1), false);
                        self.carousel_last_page_index = 1;
                    }
                }
            },
            ViewNavInput::SwipeTo(page_index) => {
                debug!("Swiped to {}", page_index);

                let Some(mut index) = self.album_index else {
                    error!("Page swiped, but no current index");
                    return;
                };

                debug!("len={}, pre index={}, pos={}", self.album.len(), index, self.carousel.position());

                if self.album.len() <= 3 {
                    // number of items in album == number of carousel page
                    self.album_index = Some(page_index as usize);
                    return;
                }

                // page_index == 0 == user has swiped to go left
                // page_index == 2 == user has swiped to go right

                // For three-page carousels (when album has more than 3 items)
                // Fotema must implement "infinite swiping". Fotema will keep
                // three items loaded to make the swiping work, the left, middle (current), and
                // right images. Awkwardly, Fotema must always return to the middle page after
                // swiping so that the next swipe to the left or right shows a "peek" at the
                // next page. However, scrolling to the middle also triggers a scrolling event
                // that must be handled to prevent unexpected scrolls in the UI.
                //
                // WARNING This is super fragile and a pain to debug! Do not touch!

                let page = self.carousel.nth_page(page_index);

                if page == self.carousel.nth_page(1) {
                    debug!("Swipe middle");
                    // If swiping from first or last element of album,
                    // then there is no rotation of carousel pages.
                    // Only index into album needs updating.
                    if index == 0 {
                        // swiped from first to second item in album.
                        index = 1;
                    } else if index == self.album.len() - 1 {
                        index -= 1;
                    }
                } else if page == self.carousel.nth_page(0) && index > 0 {
                    debug!("Swiped left");
                    //if self.carousel_last_page_index != page_index {
                    if index > 1 {
                        debug!("Rotating right");
                        self.carousel_pages.rotate_right(1);
                        self.carousel.reorder(self.carousel_pages[0].widget(), 0);
                    }

                    // Moved to left.
                    index -= 1;

                    // Pre-load item that will be visible on _next_ left swipe
                    if index > 0 {
                        self.carousel_pages[0].emit(ViewOneInput::View(self.album[index - 1].clone()));
                    }
                } else if page == self.carousel.nth_page(2) && index < self.album.len() - 1 {
                    debug!("Swiped right");
                    // If swiping to last item, then no rotation necessary.
                    if index < self.album.len() - 2 {
                        debug!("Rotating left");
                        self.carousel_pages.rotate_left(1);
                        self.carousel.reorder(self.carousel_pages[2].widget(), 2);
                    }

                    // Move to right.
                    index += 1;

                    // Pre-load item that will be visible on _next_ right swipe
                    if index < self.album.len() - 1 {
                        self.carousel_pages[2].emit(ViewOneInput::View(self.album[index + 1].clone()));
                    }
                }

                if self.carousel_last_page_index != page_index && self.carousel.position() != 1.0 && index > 1 && index < self.album.len() - 1 {
                    debug!("Repositioning to middle");
                    self.carousel.scroll_to(self.carousel_pages[1].widget(), false);
                }

                assert!(self.carousel_pages[0].widget() == &self.carousel.nth_page(0));
                assert!(self.carousel_pages[1].widget() == &self.carousel.nth_page(1));
                assert!(self.carousel_pages[2].widget() == &self.carousel.nth_page(2));

                debug!("len={}, post index={}, pos={}", self.album.len(), index, self.carousel.position());

                self.album_index = Some(index);
                self.carousel_last_page_index = page_index;
            },
            ViewNavInput::ToggleInfo => {
                self.shows_infobar = !self.shows_infobar;
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
                if self.album_index.is_some_and(|index| index > 0) {
                    self.carousel.scroll_to(&self.carousel.nth_page(0), false);
                }
            },
            ViewNavInput::GoRight => {
                let album_len = self.album.len();
                if self.album_index.is_some_and(|index| index < album_len - 1) {
                    let position = self.carousel.position() as u32 + 1;
                    if position < self.carousel.n_pages() {
                        // WARN when scrolling right the animation should be disabled to hide
                        // the glitchy flashes related to my rather janky infinite scrolling :-(
                        self.carousel.scroll_to(&self.carousel.nth_page(position), false);
                    }
                }
            },
            ViewNavInput::Adapt(adaptive::Layout::Narrow) => {
                self.is_narrow = true;
            },
            ViewNavInput::Adapt(adaptive::Layout::Wide) => {
                self.is_narrow = false;
            },
            ViewNavInput::RestoreIgnoredFaces => {
                let Some(index) = self.album_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                let visual = &self.album[index];

                info!("Restoring unknown faces for {}", visual.visual_id);

                if let Some(picture_id) = visual.picture_id {
                    if let Err(e) = self.people_repo.restore_ignored_faces(picture_id) {
                        error!("Failed restoring ignored faces: {}", e);
                    }
                }

                self.carousel_pages[self.carousel.position() as usize].emit(ViewOneInput::Refresh);
            },
            ViewNavInput::IgnoreUnknownFaces => {
                let Some(index) = self.album_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                info!("Ignoring unknown faces");

                let visual = &self.album[index];
                if let Some(picture_id) = visual.picture_id {
                    if let Err(e) = self.people_repo.ignore_unknown_faces(picture_id) {
                        error!("Failed ignoring unknown faces: {}", e);
                    }
                }

                self.carousel_pages[self.carousel.position() as usize].emit(ViewOneInput::Refresh);
            },
            ViewNavInput::ScanForFaces => {

                let Some(index) = self.album_index else {
                    return;
                };

                if index == 0 {
                    return;
                }

                info!("Scan for more faces");

                let visual = &self.album[index];
                if let Some(picture_id) = visual.picture_id {
                    let _ = sender.output(ViewNavOutput::ScanForFaces(picture_id));
                }
            },
        }
    }
}

impl ViewNav {
    fn is_left_button_sensitive(&self) -> bool {
        self.album_index.is_some_and(|index| index > 0)
    }

    fn is_right_button_sensitive(&self) -> bool {
        !self.album.is_empty()
            && self.album_index.is_some_and(|index| index != self.album.len() - 1)
    }
}
