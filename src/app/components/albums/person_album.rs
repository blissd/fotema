// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use fotema_core::VisualId;
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::binding::*;
use relm4::adw;
use relm4::gtk::gdk;
use relm4::actions::{RelmAction, RelmActionGroup};

use crate::app::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;
use crate::app::components::albums:: {
    album::{Album, AlbumInput, AlbumOutput},
    album_filter::AlbumFilter,
};

use fotema_core::people;
use fotema_core::PictureId;
use crate::fl;

use tracing::info;

const NARROW_EDGE_LENGTH: i32 = 50;
const WIDE_EDGE_LENGTH: i32 = 200;

relm4::new_action_group!(PersonActionGroup, "person");

// Rename a person
relm4::new_stateless_action!(RenameAction, PersonActionGroup, "rename");

// Delete a person
relm4::new_stateless_action!(DeleteAction, PersonActionGroup, "delete");

#[derive(Debug)]
pub enum PersonAlbumInput {

    /// Album is visible
    Activate,

    // State has been updated
    Refresh,

    /// View album for a person
    View(people::Person),

    /// Adapt to layout
    Adapt(adaptive::Layout),

    /// Underlying album has scrolled
    ScrollOffset(f64),

    /// Picture selected in underlying album
    Selected(VisualId),

    /// Rename person
    Rename,

    /// Delete person
    Delete,
}

#[derive(Debug)]
pub enum PersonAlbumOutput {
    /// User has selected photo or video in grid view
    Selected(VisualId, AlbumFilter),
}

pub struct PersonAlbum {
    repo: people::Repository,
    person: Option<people::Person>,
    picture_ids: Vec<PictureId>,
    album: Controller<Album>,
    avatar: adw::Avatar,
    title: gtk::Label,
    active_view: ActiveView,
    edge_length: I32Binding,
}

#[relm4::component(pub)]
impl SimpleComponent for PersonAlbum {
    type Init = (SharedState, people::Repository, ActiveView);
    type Input = PersonAlbumInput;
    type Output = PersonAlbumOutput;

    menu! {
        primary_menu: {
            section! {
                // FIXME I would like to have the person's name in these menu items.
                &fl!("person-menu-rename") => RenameAction,
                &fl!("person-menu-delete") => DeleteAction,
            }
        }
    }

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                #[wrap(Some)]
                #[local_ref]
                set_title_widget = &title -> gtk::Label {
                    add_css_class: "title",
                },

                pack_end = &gtk::MenuButton {
                    set_icon_name: "open-menu-symbolic",
                    set_menu_model: Some(&primary_menu),
                },
            },

            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_vexpand: true,
                set_spacing: 12,

                #[local_ref]
                avatar -> adw::Avatar,

                model.album.widget(),
            }
        }
    }

    fn init(
        (state, repo, active_view): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let avatar = adw::Avatar::builder()
            .size(NARROW_EDGE_LENGTH)
            .show_initials(true)
            .build();

        let album = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Person, AlbumFilter::None))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, _) => PersonAlbumInput::Selected(id),
                AlbumOutput::ScrollOffset(offset) => PersonAlbumInput::ScrollOffset(offset),
            });

        let title = gtk::Label::builder()
            .build();

        let model = PersonAlbum {
            repo,
            person: None,
            avatar: avatar.clone(),
            title: title.clone(),
            album,
            active_view,
            picture_ids: vec![],
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
        };

        model.avatar.add_write_only_binding(&model.edge_length, "size");
        let widgets = view_output!();

        let mut actions = RelmActionGroup::<PersonActionGroup>::new();

        let rename_action = {
            let sender = sender.clone();
            RelmAction::<RenameAction>::new_stateless(move |_| {
                let _ = sender.input(PersonAlbumInput::Rename);
            })
        };

        let delete_action = {
            let sender = sender.clone();
            RelmAction::<DeleteAction>::new_stateless(move |_| {
                let _ = sender.input(PersonAlbumInput::Delete);
            })
        };

        actions.add_action(rename_action);
        actions.add_action(delete_action);
        actions.register_for_widget(root);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PersonAlbumInput::Activate => {
                *self.active_view.write() = ViewName::Person;
                self.album.sender().emit(AlbumInput::Activate);
            }
            PersonAlbumInput::Refresh => {
                self.album.sender().emit(AlbumInput::Refresh);
            }
            PersonAlbumInput::View(person) => {
                info!("Viewing album for person: {}", person.person_id);

                let img = gdk::Texture::from_filename(&person.thumbnail_path).ok();
                self.avatar.set_custom_image(img.as_ref());
                self.avatar.set_text(Some(&person.name));

                if !self.avatar.is_visible() {
                    self.avatar.set_visible(true);
                }

                self.picture_ids = self.repo.find_pictures_for_person(person.person_id).unwrap_or(vec![]);
                info!("Person {} has {} items to view.", person.person_id, self.picture_ids.len());
                self.album.sender().emit(AlbumInput::Activate);
                self.album.sender().emit(AlbumInput::Filter(AlbumFilter::Any(self.picture_ids.clone())));
                self.album.sender().emit(AlbumInput::GoToFirst);

                self.title.set_label(&person.name);
                self.person = Some(person);
            }
            PersonAlbumInput::Selected(visual_id) => {
                let _ = sender.output(PersonAlbumOutput::Selected(visual_id, AlbumFilter::Any(self.picture_ids.clone())));
            },
            PersonAlbumInput::Adapt(layout @ adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
                // FIXME album should directly subscribe to layout state.
                self.album.sender().emit(AlbumInput::Adapt(layout));
            },
            PersonAlbumInput::Adapt(layout @ adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
                // FIXME album should directly subscribe to layout state.
                self.album.sender().emit(AlbumInput::Adapt(layout));
            },
            PersonAlbumInput::ScrollOffset(offset) => {
                if offset >= 210.0 && self.avatar.is_visible() {
                    self.avatar.set_visible(false);
                } else if offset == 0.0 && !self.avatar.is_visible() {
                    self.avatar.set_visible(true);
                }
            },
            PersonAlbumInput::Rename => {
                info!("Renaming person");
            },
            PersonAlbumInput::Delete => {
                info!("Deleting person");
            },
        }
    }
}

