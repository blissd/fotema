// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gio, gdk, gdk_pixbuf};
use relm4::*;
use relm4::prelude::*;
use relm4::actions::{RelmAction, RelmActionGroup};


use crate::fl;
use fotema_core::people;
use fotema_core::PictureId;
use fotema_core::FaceId;
use fotema_core::PersonId;

use tracing::{debug, error, info};

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

            #[local_ref]
            people -> gtk::ListBox,
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
            .primary_icon_name("reaction-add-symbolic")
            .build();

        let people = gtk::ListBox::builder()
            .build();

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
            },
            PersonSelectInput::NewPerson(face_id, thumbnail) => {
                debug!("Face {} is a new person", face_id);
                let name = self.face_name.text().to_string();
                if let Err(e) = self.people_repo.add_person(face_id, &thumbnail, &name) {
                    error!("Failed adding new person: {:?}", e);
                }
            },
            PersonSelectInput::Activate(face_id, thumbnail) => {
                println!("set person for face {}", face_id);

                {
                    let sender = sender.clone();
                    let thumbnail = thumbnail.clone();
                    self.face_name.connect_activate(move |_| {
                        println!("Activated");
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

                    let label = gtk::Label::new(Some(&person.name));

                    let row_box = gtk::Box::builder()
                        .orientation(gtk::Orientation::Horizontal)
                        .build();

                    row_box.append(&avatar);
                    row_box.append(&label);

                    let row = gtk::ListBoxRow::builder()
                        .child(&row_box)
                        .activatable(true)
                        .build();

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
