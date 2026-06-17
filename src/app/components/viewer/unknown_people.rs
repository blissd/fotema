// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use relm4::adw::{self, prelude::*};
use relm4::binding::*;
use relm4::gtk::{self, gdk};
use relm4::prelude::*;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::RelmObjectExt;

use crate::adaptive;
use crate::fl;
use fotema_core::FaceId;
use fotema_core::people;

use super::person_select::{PersonSelect, PersonSelectInput, PersonSelectOutput};

use tracing::{debug, error};

use std::path::PathBuf;

// Face avatar edge length, matching the People album so the two grids look the
// same. Deliberately larger than the 64px per-picture overlay so unfamiliar
// faces are easy to recognise while naming them.
const NARROW_EDGE_LENGTH: i32 = 170;
const WIDE_EDGE_LENGTH: i32 = 200;

/// A single unnamed face shown in the grid.
pub struct UnknownFaceItem {
    face: people::Face,

    // Length of avatar edge, bound so the grid resizes when the layout changes.
    edge_length: I32Binding,
}

pub struct UnknownFaceWidgets {
    avatar: adw::Avatar,

    // If the avatar has been bound to edge_length.
    is_bound: bool,
}

impl RelmGridItem for UnknownFaceItem {
    type Root = gtk::Box;
    type Widgets = UnknownFaceWidgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            root = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 4,
                set_halign: gtk::Align::Center,
                set_valign: gtk::Align::Center,

                #[name(avatar)]
                adw::Avatar {
                    set_size: NARROW_EDGE_LENGTH,
                    set_show_initials: false,
                }
            }
        }

        let widgets = UnknownFaceWidgets {
            avatar,
            is_bound: false,
        };
        (root, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        // Bind once: re-binding leaks GWeakRefs and eventually aborts (see the
        // same guard in people_album.rs).
        if !widgets.is_bound {
            widgets
                .avatar
                .add_write_only_binding(&self.edge_length, "size");
            widgets.is_bound = true;
        }

        let img = gdk::Texture::from_filename(&self.face.thumbnail_path).ok();
        widgets.avatar.set_custom_image(img.as_ref());
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.avatar.set_custom_image(None::<&gdk::Paintable>);
    }
}

#[derive(Debug)]
pub enum UnknownPeopleInput {
    /// Reload the unnamed-faces grid (clustered, most-frequent first).
    Refresh,

    /// The grid selection changed; show the selected face(s) in the naming
    /// sidebar. Several faces can be selected (Ctrl/Shift+click) and named at
    /// once.
    SelectionChanged,

    /// The naming sidebar finished associating/creating a person.
    PersonSelected,

    /// Adapt avatar size to the window layout.
    Adapt(adaptive::Layout),
}

#[derive(Debug)]
pub enum UnknownPeopleOutput {}

pub struct UnknownPeople {
    people_repo: people::Repository,

    face_grid: TypedGridView<UnknownFaceItem, gtk::MultiSelection>,

    /// Scrolled grid of unnamed faces; hidden (in favour of `status`) when there
    /// are none left to name.
    avatars: gtk::ScrolledWindow,

    /// Empty state shown when every face has been named.
    status: adw::StatusPage,

    /// Shared avatar edge length, updated on layout changes.
    edge_length: I32Binding,

    /// Always-visible naming sidebar on the right. Reuses the same person
    /// selector as the photo viewer, just relocated from a bottom sheet to a
    /// persistent column so naming many faces in a row is quick.
    person_select: AsyncController<PersonSelect>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for UnknownPeople {
    type Init = people::Repository;
    type Input = UnknownPeopleInput;
    type Output = UnknownPeopleOutput;

    view! {
        adw::OverlaySplitView {
            set_collapsed: false,
            set_sidebar_position: gtk::PackType::End,
            set_min_sidebar_width: 320.0,
            set_max_sidebar_width: 360.0,

            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                avatars -> gtk::ScrolledWindow {
                    set_hexpand: true,
                    set_vexpand: true,

                    #[local_ref]
                    grid_view -> gtk::GridView {
                        // Remove 'view' css class to avoid black background.
                        remove_css_class: "view",
                        set_orientation: gtk::Orientation::Vertical,
                        // Multi-selection: plain click selects one, Ctrl/Shift+click
                        // extend the selection so many faces can be named at once.
                    },
                },

                #[local_ref]
                status -> adw::StatusPage {
                    set_valign: gtk::Align::Start,
                    set_vexpand: true,
                    set_visible: false,
                    set_icon_name: Some("sentiment-very-satisfied-symbolic"),
                },
            },

            #[wrap(Some)]
            set_sidebar = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                person_select_widget -> gtk::Box {},
            },
        }
    }

    async fn init(
        people_repo: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let face_grid: TypedGridView<UnknownFaceItem, gtk::MultiSelection> = TypedGridView::new();
        let grid_view = &face_grid.view.clone();

        {
            let sender = sender.clone();
            face_grid
                .selection_model
                .connect_selection_changed(move |_, _, _| {
                    sender.input(UnknownPeopleInput::SelectionChanged);
                });
        }

        let avatars = gtk::ScrolledWindow::builder().build();

        let status = adw::StatusPage::new();
        status.set_title(&fl!("faces-page-empty", "title"));
        status.set_description(Some(&fl!("faces-page-empty", "description")));

        let person_select = PersonSelect::builder().launch(people_repo.clone()).forward(
            sender.input_sender(),
            |msg| match msg {
                PersonSelectOutput::Done => UnknownPeopleInput::PersonSelected,
            },
        );
        let person_select_widget = person_select.widget().clone();

        let widgets = view_output!();

        let model = Self {
            people_repo,
            face_grid,
            avatars,
            status,
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
            person_select,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            UnknownPeopleInput::Refresh => {
                self.face_grid.clear();

                // Query + greedy O(n²) clustering of ~10k faces (512-dim) is
                // heavy: run it off the UI thread so the main loop keeps
                // responding and GNOME never flags the app as "not responding".
                let repo = self.people_repo.clone();
                let faces = match relm4::spawn_blocking(move || {
                    repo.find_unnamed_faces().map(|faces| {
                        faces
                            .into_iter()
                            .filter(|face| face.thumbnail_path.exists())
                            .collect::<Vec<_>>()
                    })
                })
                .await
                {
                    Ok(Ok(faces)) => faces,
                    Ok(Err(e)) => {
                        error!("Failed getting unnamed faces: {}", e);
                        Vec::new()
                    }
                    Err(e) => {
                        error!("Unnamed-faces task failed: {}", e);
                        Vec::new()
                    }
                };

                debug!("Found {} unnamed faces", faces.len());
                let edge = self.edge_length.clone();
                self.face_grid
                    .extend_from_iter(faces.into_iter().map(|face| UnknownFaceItem {
                        face,
                        edge_length: edge.clone(),
                    }));

                // Show the empty state when nothing is left to name.
                let is_empty = self.face_grid.len() == 0;
                self.avatars.set_visible(!is_empty);
                self.status.set_visible(is_empty);
            }
            UnknownPeopleInput::SelectionChanged => {
                // Collect all currently selected faces (in grid order).
                let mut face_ids: Vec<FaceId> = Vec::new();
                let mut first_thumb: Option<PathBuf> = None;
                for position in 0..self.face_grid.len() {
                    if self.face_grid.selection_model.is_selected(position) {
                        if let Some(item) = self.face_grid.get(position) {
                            let face = &item.borrow().face;
                            face_ids.push(face.face_id);
                            if first_thumb.is_none() {
                                first_thumb = Some(face.thumbnail_path.clone());
                            }
                        }
                    }
                }
                debug!("{} face(s) selected", face_ids.len());
                if let Some(thumbnail) = first_thumb {
                    self.person_select
                        .emit(PersonSelectInput::ActivateMany(face_ids, thumbnail));
                }
            }
            UnknownPeopleInput::PersonSelected => {
                // The named face is no longer unnamed: reload so it drops out
                // and the next-most-frequent unknown bubbles up.
                sender.input(UnknownPeopleInput::Refresh);
            }
            UnknownPeopleInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            }
            UnknownPeopleInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            }
        }
    }
}
