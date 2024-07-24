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
    NewPerson(PathBuf),

    /// Associate a face with a person. Used when user selects person with return key.
    Associate(PersonId),

    /// Associate a face with a person. Used when user clicks person with mouse.
    /// usize is index into vector of people
    AssociateByIndex(usize),
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

    /// List of avatars for people
    people_list: gtk::ListBox,

    /// List of person IDs of people.
    /// MUST be in same order as people_list.
    all_people: Vec<PersonId>,

    /// ID of face to associate with person,
    face_id: Option<FaceId>,
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
                people_list -> gtk::ListBox,
            }
        }
    }

    async fn init(
        people_repo: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let face_thumbnail = adw::Avatar::builder()
            .size(100)
            .show_initials(false)
            .build();

        let face_name = gtk::Entry::builder()
            .placeholder_text(fl!("people-person-search", "placeholder"))
            .build();

        let people_list = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .activate_on_single_click(true)
            .build();

        {
            let people_list2 = people_list.clone();
            people_list.connect_row_activated(move |_, row| {
                debug!("activated = {:?}", row);
                if let Some(index) = people_list2.index_of_child(row) {
                    if index >= 0 {
                        sender.input(PersonSelectInput::AssociateByIndex(index as usize));
                    } else {
                        error!("Invalid vector index: {}", index);
                    }
                }
            });
        }

        people_list.connect_row_selected(|_, row| {
            debug!("selected = {:?}", row);
        });

        let widgets = view_output!();

        let model = Self {
            people_repo,
            face_thumbnail,
            face_name,
            people_list,
            all_people: vec![],
            face_id: None,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            PersonSelectInput::Activate(face_id, thumbnail) => {
                debug!("Set person for face {}", face_id);

                self.people_list.remove_all();
                self.all_people.clear();
                self.face_name.set_text("");
                self.face_id = Some(face_id);

                {
                    let sender = sender.clone();
                    let thumbnail = thumbnail.clone();
                    self.face_name.connect_activate(move |_| {
                        debug!("Face name entry activated.");
                        sender.input(PersonSelectInput::NewPerson(thumbnail.clone()));
                    });
                }

                let img = gdk::Texture::from_filename(&thumbnail).ok();
                self.face_thumbnail.set_custom_image(img.as_ref());

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
                            sender.input(PersonSelectInput::Associate(person.person_id));
                        });
                    }

                    self.people_list.append(&row);
                    self.all_people.push(person.person_id);
                }
            },
            PersonSelectInput::Associate(person_id) => {
                if let Some(face_id) = self.face_id {
                    debug!("Associating face {} with person {}", face_id, person_id);
                    if let Err(e) = self.people_repo.mark_as_person(face_id, person_id) {
                        error!("Failed associating face with person: {:?}", e);
                    }
                }
                self.people_list.remove_all();
                self.all_people.clear();
                let _ = sender.output(PersonSelectOutput::Done);
            },
            PersonSelectInput::AssociateByIndex(person_id_index) => {
                if let (Some(face_id), Some(person_id)) = (self.face_id, self.all_people.get(person_id_index)) {
                    debug!("Associating face {} with person {} by idnex", face_id, person_id);
                    if let Err(e) = self.people_repo.mark_as_person(face_id, *person_id) {
                        error!("Failed associating face with person: {:?}", e);
                    }
                }
                self.people_list.remove_all();
                self.all_people.clear();
                let _ = sender.output(PersonSelectOutput::Done);
            },
            PersonSelectInput::NewPerson(thumbnail) => {
                if let Some(face_id) = self.face_id {
                    debug!("Face {} is a new person", face_id);
                    let name = self.face_name.text().to_string();
                    if let Err(e) = self.people_repo.add_person(face_id, &thumbnail, &name) {
                        error!("Failed adding new person: {:?}", e);
                    }
                }
                self.people_list.remove_all();
                self.all_people.clear();
                let _ = sender.output(PersonSelectOutput::Done);
            },
        }
    }
}
