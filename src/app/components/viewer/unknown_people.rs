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
use crate::app::SettingsState;
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

    /// The user pressed "find matches": ask the app to run a background
    /// recognition pass, then reload the grid when it finishes.
    Recognize,

    /// Act on the selected face(s): ignore them in the normal view, or restore
    /// them in the ignored view.
    ActOnSelected,

    /// Toggle between the normal (unnamed) grid and the ignored-faces grid.
    ShowIgnoredFaces(bool),

    /// Auto-recognition setting changed: hide the manual "find matches" button
    /// while it is on (recognition then runs automatically after naming).
    AutoRecognitionChanged(bool),

    /// Adapt avatar size to the window layout.
    Adapt(adaptive::Layout),
}

#[derive(Debug)]
pub enum UnknownPeopleOutput {
    /// The user pressed "find matches": run a background recognition pass
    /// (always, regardless of the auto-recognition setting).
    RecognizeRequested,

    /// A face was just named: run a background recognition pass *if* the user
    /// has auto-recognition enabled in the settings.
    AutoRecognizeRequested,
}

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

    /// Faces currently selected in the grid (for the ignore/restore action).
    selected_faces: Vec<FaceId>,

    /// Action button on the selection: "ignore face(s)" normally, "restore
    /// face(s)" while the ignored view is shown. Enabled only with a selection.
    ignore_button: gtk::Button,

    /// Whether the grid currently shows ignored faces instead of unnamed ones.
    show_ignored: bool,
    /// Toggle for the above; hidden when there is nothing ignored to restore.
    show_ignored_toggle: gtk::ToggleButton,

    /// Manual "find matches" button; hidden while automatic recognition is on
    /// (it would be redundant — naming then triggers recognition itself).
    recognize_button: gtk::Button,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for UnknownPeople {
    type Init = (people::Repository, SettingsState);
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

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::End,
                    set_margin_top: 6,
                    set_margin_end: 6,

                    #[local_ref]
                    show_ignored_toggle -> gtk::ToggleButton {
                        set_label: &fl!("unknown-people-show-ignored"),
                        add_css_class: "flat",
                        set_visible: false,
                        connect_toggled[sender] => move |btn| {
                            sender.input(UnknownPeopleInput::ShowIgnoredFaces(btn.is_active()));
                        },
                    },
                },

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
                recognize_button -> gtk::Button {
                    set_label: &fl!("unknown-people-recognize"),
                    set_tooltip_text: Some(&fl!("unknown-people-recognize", "tooltip")),
                    add_css_class: "pill",
                    add_css_class: "suggested-action",
                    set_halign: gtk::Align::Center,
                    set_margin_top: 8,
                    set_margin_bottom: 4,
                    connect_clicked => UnknownPeopleInput::Recognize,
                },

                #[local_ref]
                person_select_widget -> gtk::Box {},

                #[local_ref]
                ignore_button -> gtk::Button {
                    set_label: &fl!("unknown-people-ignore-face"),
                    set_tooltip_text: Some(&fl!("unknown-people-ignore-face", "tooltip")),
                    add_css_class: "flat",
                    set_sensitive: false,
                    set_margin_top: 4,
                    set_margin_bottom: 8,
                    set_margin_start: 8,
                    set_margin_end: 8,
                    connect_clicked => UnknownPeopleInput::ActOnSelected,
                },
            },
        }
    }

    async fn init(
        (people_repo, settings_state): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        // Track the auto-recognition setting so the manual button can hide when
        // it is on.
        let auto_recognition = settings_state.read().face_recognition_auto;
        settings_state.subscribe(sender.input_sender(), |settings| {
            UnknownPeopleInput::AutoRecognitionChanged(settings.face_recognition_auto)
        });

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

        let ignore_button = gtk::Button::new();

        let show_ignored_toggle = gtk::ToggleButton::new();

        // Hidden while auto-recognition is on (it would be redundant then).
        let recognize_button = gtk::Button::new();
        recognize_button.set_visible(!auto_recognition);

        let widgets = view_output!();

        let model = Self {
            people_repo,
            face_grid,
            avatars,
            status,
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
            person_select,
            selected_faces: vec![],
            ignore_button: ignore_button.clone(),
            show_ignored: false,
            show_ignored_toggle: show_ignored_toggle.clone(),
            recognize_button: recognize_button.clone(),
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            UnknownPeopleInput::Refresh => {
                // Remember the scroll position so naming/ignoring a face doesn't
                // bounce the user to the top — they carry on roughly where they
                // were. Captured before clearing (which resets the adjustment).
                let scroll = self.avatars.vadjustment().value();

                self.face_grid.clear();

                // Query + greedy O(n²) clustering of ~10k faces (512-dim) is
                // heavy: run it off the UI thread so the main loop keeps
                // responding and GNOME never flags the app as "not responding".
                let repo = self.people_repo.clone();
                let show_ignored = self.show_ignored;
                let (faces, has_ignored) = relm4::spawn_blocking(move || {
                    let exists = |faces: Vec<people::Face>| {
                        faces
                            .into_iter()
                            .filter(|face| face.thumbnail_path.exists())
                            .collect::<Vec<_>>()
                    };
                    if show_ignored {
                        let faces = exists(repo.find_ignored_faces().unwrap_or_default());
                        let has_ignored = !faces.is_empty();
                        (faces, has_ignored)
                    } else {
                        let faces = exists(repo.find_unnamed_faces().unwrap_or_default());
                        let has_ignored = repo
                            .find_ignored_faces()
                            .map(|v| !v.is_empty())
                            .unwrap_or(false);
                        (faces, has_ignored)
                    }
                })
                .await
                .unwrap_or_else(|e| {
                    error!("Faces task failed: {}", e);
                    (Vec::new(), false)
                });

                debug!(
                    "Found {} faces (ignored view: {})",
                    faces.len(),
                    self.show_ignored
                );

                // If the last ignored face was just restored, drop back to the
                // normal view rather than showing an empty ignored list.
                if self.show_ignored && !has_ignored {
                    self.show_ignored = false;
                    sender.input(UnknownPeopleInput::Refresh);
                    return;
                }

                let edge = self.edge_length.clone();
                self.face_grid
                    .extend_from_iter(faces.into_iter().map(|face| UnknownFaceItem {
                        face,
                        edge_length: edge.clone(),
                    }));

                // Offer the "show ignored" toggle only when there's something to
                // restore (or we're already viewing it).
                self.show_ignored_toggle
                    .set_visible(has_ignored || self.show_ignored);
                if self.show_ignored_toggle.is_active() != self.show_ignored {
                    self.show_ignored_toggle.set_active(self.show_ignored);
                }

                // Empty state, worded for whichever view is active.
                let is_empty = self.face_grid.len() == 0;
                self.avatars.set_visible(!is_empty);
                self.status.set_visible(is_empty);
                if self.show_ignored {
                    self.status
                        .set_title(&fl!("unknown-people-no-ignored", "title"));
                    self.status
                        .set_description(Some(&fl!("unknown-people-no-ignored", "description")));
                } else {
                    self.status.set_title(&fl!("faces-page-empty", "title"));
                    self.status
                        .set_description(Some(&fl!("faces-page-empty", "description")));
                }

                // Restore the remembered position once the new grid is laid out.
                if scroll > 0.0 {
                    let vadj = self.avatars.vadjustment();
                    gtk::glib::idle_add_local_once(move || vadj.set_value(scroll));
                }
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
                self.selected_faces = face_ids.clone();
                self.ignore_button.set_sensitive(!face_ids.is_empty());
                if let Some(thumbnail) = first_thumb {
                    self.person_select
                        .emit(PersonSelectInput::ActivateMany(face_ids, thumbnail));
                }
            }
            UnknownPeopleInput::ActOnSelected => {
                if self.selected_faces.is_empty() {
                    return;
                }
                let mut repo = self.people_repo.clone();
                if self.show_ignored {
                    debug!("Restoring {} face(s)", self.selected_faces.len());
                    for face_id in &self.selected_faces {
                        if let Err(e) = repo.restore_face(*face_id) {
                            error!("Failed to restore face {}: {}", face_id, e);
                        }
                    }
                } else {
                    debug!("Ignoring {} face(s)", self.selected_faces.len());
                    for face_id in &self.selected_faces {
                        if let Err(e) = repo.mark_ignore(*face_id) {
                            error!("Failed to ignore face {}: {}", face_id, e);
                        }
                    }
                }
                self.selected_faces.clear();
                self.ignore_button.set_sensitive(false);
                // The acted-on faces leave the current view: reload (keeps scroll).
                sender.input(UnknownPeopleInput::Refresh);
            }
            UnknownPeopleInput::ShowIgnoredFaces(show) => {
                if self.show_ignored == show {
                    return;
                }
                self.show_ignored = show;
                // The selection action becomes "restore" in the ignored view.
                if show {
                    self.ignore_button
                        .set_label(&fl!("unknown-people-restore-face"));
                    self.ignore_button
                        .set_tooltip_text(Some(&fl!("unknown-people-restore-face", "tooltip")));
                } else {
                    self.ignore_button
                        .set_label(&fl!("unknown-people-ignore-face"));
                    self.ignore_button
                        .set_tooltip_text(Some(&fl!("unknown-people-ignore-face", "tooltip")));
                }
                self.selected_faces.clear();
                self.ignore_button.set_sensitive(false);
                sender.input(UnknownPeopleInput::Refresh);
            }
            UnknownPeopleInput::PersonSelected => {
                // The named face is no longer unnamed: reload so it drops out
                // and the next-most-frequent unknown bubbles up.
                sender.input(UnknownPeopleInput::Refresh);
                // Auto-propagate (only if enabled in settings): run a background
                // recognition pass so the just-named person picks up their other
                // faces automatically.
                let _ = sender.output(UnknownPeopleOutput::AutoRecognizeRequested);
            }
            UnknownPeopleInput::Recognize => {
                // Manual trigger (button): same background pass, on demand,
                // regardless of the auto-recognition setting.
                let _ = sender.output(UnknownPeopleOutput::RecognizeRequested);
            }
            UnknownPeopleInput::AutoRecognitionChanged(auto) => {
                // Hide the manual button while auto-recognition is on.
                self.recognize_button.set_visible(!auto);
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
