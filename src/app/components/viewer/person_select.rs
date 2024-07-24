// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gdk};
use relm4::*;
use relm4::prelude::*;


use crate::fl;
use fotema_core::people;
use fotema_core::FaceId;
use fotema_core::PersonId;

use tracing::{debug, error};

use std::path::PathBuf;

#[derive(Debug)]
pub enum PersonSelectInput {
    /// Present person selector for a give face.
    Activate(FaceId, PathBuf),

    /// Create a new person to associate with a face.
    NewPerson(FaceId, PathBuf),

    /// Associate a face with a person.
    Associate(FaceId, PersonId),
}

#[derive(Debug)]
pub enum PersonSelectOutput {
    /// Face and person association either completed or dismissed.
    Done,
}

pub struct PersonSelect {
    people_repo: people::Repository,
    face_thumbnail: adw::Avatar,
    face_name: gtk::Entry,
    people: gtk::ListBox,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for PersonSelect {
    type Init = people::Repository;
    type Input = PersonSelectInput;
    type Output = PersonSelectOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,
            set_margin_all: 12,

            #[local_ref]
            face_thumbnail -> adw::Avatar,

            #[local_ref]
            face_name -> gtk::Entry,

            gtk::ScrolledWindow {
                set_vexpand: true,

                #[local_ref]
                people -> gtk::ListBox,
            }
        }
    }

    async fn init(
        people_repo: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let face_thumbnail = adw::Avatar::builder()
            .size(100)
            .show_initials(false)
            .build();

        let face_name = gtk::Entry::builder()
            .placeholder_text(fl!("people-person-search", "placeholder"))
            .build();

        let people = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .activate_on_single_click(true)
            .build();

        people.connect_row_activated(|_, row| {
            debug!("activated = {:?}", row);
        });

        people.connect_row_selected(|_, row| {
            debug!("selected = {:?}", row);
        });

        let widgets = view_output!();

        let model = Self {
            people_repo,
            face_thumbnail,
            face_name,
            people,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            PersonSelectInput::Associate(face_id, person_id) => {
                debug!("Associating face {} with person {}", face_id, person_id);
                if let Err(e) = self.people_repo.mark_as_person(face_id, person_id) {
                    error!("Failed associating face with person: {:?}", e);
                }
                let _ = sender.output(PersonSelectOutput::Done);
            },
            PersonSelectInput::NewPerson(face_id, thumbnail) => {
                debug!("Face {} is a new person", face_id);
                let name = self.face_name.text().to_string();
                if let Err(e) = self.people_repo.add_person(face_id, &thumbnail, &name) {
                    error!("Failed adding new person: {:?}", e);
                }
                let _ = sender.output(PersonSelectOutput::Done);
            },
            PersonSelectInput::Activate(face_id, thumbnail) => {
                debug!("Set person for face {}", face_id);

                {
                    let sender = sender.clone();
                    let thumbnail = thumbnail.clone();
                    self.face_name.connect_activate(move |_| {
                        debug!("Face name entry activated.");
                        sender.input(PersonSelectInput::NewPerson(face_id, thumbnail.clone()));
                    });
                }

                let img = gdk::Texture::from_filename(&thumbnail).ok();
                self.face_thumbnail.set_custom_image(img.as_ref());

                self.people.remove_all();
                self.face_name.set_text("");

                let people = self.people_repo.all_people().unwrap_or(vec![]);

                for person in people {
                    let avatar = adw::Avatar::builder()
                        .size(50)
                        .name(&person.name)
                        .build();

                    let img = gdk::Texture::from_filename(&person.thumbnail_path).ok();
                    avatar.set_custom_image(img.as_ref());

                    let row = adw::ActionRow::builder()
                        .title(person.name)
                        .activatable(true)
                        .build();

                    row.add_prefix(&avatar);

                    {
                        let sender = sender.clone();
                        row.connect_activate(move |_| {
                            sender.input(PersonSelectInput::Associate(face_id, person.person_id));
                        });
                    }

                    self.people.append(&row);
                }
            },
        }
    }
}
