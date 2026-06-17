// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gdk};
use relm4::prelude::*;
use relm4::*;

use crate::fl;
use fotema_core::FaceId;
use fotema_core::PersonId;
use fotema_core::people;

use tracing::{debug, error};

use std::path::PathBuf;

#[derive(Debug)]
pub enum PersonSelectInput {
    /// Present person selector for a give face.
    Activate(FaceId, PathBuf),

    /// Create a new person to associate with a face.
    NewPerson,

    /// Associate a face with a person. Used when user selects person with return key.
    Associate(PersonId),

    /// Associate a face with a person. Used when user clicks person with mouse.
    /// usize is index into vector of people
    AssociateByIndex(usize),

    /// Complete the name entry to the best-matching known name (Tab key).
    Autocomplete,
}

#[derive(Debug)]
pub enum PersonSelectOutput {
    /// Face and person association either completed or dismissed.
    Done,
}

pub struct PersonSelect {
    people_repo: people::Repository,

    /// Avatar for face to associate with person.
    avatar: adw::Avatar,

    /// Input box for new person name... and search, if I can get it to work.
    face_name: gtk::Entry,

    /// List of avatars for people
    people_list: gtk::ListBox,

    /// List of person IDs of people.
    /// MUST be in same order as people_list.
    all_people: Vec<PersonId>,

    /// Names of people, same order as `all_people`. Used for Tab autocomplete.
    all_names: Vec<String>,

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
            avatar -> adw::Avatar,

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
    ) -> AsyncComponentParts<Self> {
        let avatar = adw::Avatar::builder()
            .size(100)
            .show_initials(false)
            .build();

        let face_name = gtk::Entry::builder()
            .placeholder_text(fl!("people-person-search", "placeholder"))
            .input_purpose(gtk::InputPurpose::Name)
            .build();

        let people_list = gtk::ListBox::builder()
            .css_classes(["boxed-list"])
            .activate_on_single_click(true)
            .build();

        {
            let people_list2 = people_list.clone();
            let sender = sender.clone();
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

        // Suggest already-known names: live-filter the people list to those
        // whose name contains what the user is typing.
        people_list.set_filter_func({
            let face_name = face_name.clone();
            move |row| {
                let query = face_name.text().to_lowercase();
                let query = query.trim();
                if query.is_empty() {
                    return true;
                }
                row.downcast_ref::<adw::ActionRow>()
                    .map(|r| r.title().to_lowercase().contains(query))
                    .unwrap_or(true)
            }
        });
        {
            let people_list = people_list.clone();
            face_name.connect_changed(move |_| people_list.invalidate_filter());
        }

        // Enter creates a person with the typed name (or reuses a matching one).
        {
            let sender = sender.clone();
            face_name.connect_activate(move |_| {
                debug!("Face name entry activated.");
                sender.input(PersonSelectInput::NewPerson);
            });
        }

        // Tab completes the entry to the best-matching known name (autocomplete),
        // instead of moving focus.
        {
            let key_controller = gtk::EventControllerKey::new();
            let sender = sender.clone();
            key_controller.connect_key_pressed(move |_, key, _, _| {
                if key == gdk::Key::Tab || key == gdk::Key::ISO_Left_Tab {
                    sender.input(PersonSelectInput::Autocomplete);
                    gtk::glib::Propagation::Stop
                } else {
                    gtk::glib::Propagation::Proceed
                }
            });
            face_name.add_controller(key_controller);
        }

        let widgets = view_output!();

        let model = Self {
            people_repo,
            avatar,
            face_name,
            people_list,
            all_people: vec![],
            all_names: vec![],
            face_id: None,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            PersonSelectInput::Activate(face_id, thumbnail) => {
                debug!("Activate for face {}", face_id);

                self.people_list.remove_all();
                self.all_people.clear();
                self.all_names.clear();
                self.face_name.set_text("");
                self.face_id = Some(face_id);

                let img = gdk::Texture::from_filename(&thumbnail).ok();
                self.avatar.set_custom_image(img.as_ref());

                let people = self.people_repo.all_people().unwrap_or_default();

                for person in people {
                    let avatar = adw::Avatar::builder().size(50).name(&person.name).build();

                    if let Some(thumbnail_path) = person.small_thumbnail_path {
                        let img = gdk::Texture::from_filename(&thumbnail_path).ok();
                        avatar.set_custom_image(img.as_ref());
                    }

                    self.all_names.push(person.name.clone());

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
            }
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
            }
            PersonSelectInput::AssociateByIndex(person_id_index) => {
                if let (Some(face_id), Some(person_id)) =
                    (self.face_id, self.all_people.get(person_id_index))
                {
                    debug!(
                        "Associating face {} with person {} by idnex",
                        face_id, person_id
                    );
                    if let Err(e) = self.people_repo.mark_as_person(face_id, *person_id) {
                        error!("Failed associating face with person: {:?}", e);
                    }
                }
                self.people_list.remove_all();
                self.all_people.clear();
                let _ = sender.output(PersonSelectOutput::Done);
            }
            PersonSelectInput::Autocomplete => {
                let text = self.face_name.text().to_string();
                let query = text.trim().to_lowercase();
                if query.is_empty() {
                    return;
                }
                // Prefer a name that starts with what's typed; otherwise the
                // first that contains it (matches the visible filtered list).
                let best = self
                    .all_names
                    .iter()
                    .find(|n| n.to_lowercase().starts_with(&query))
                    .or_else(|| self.all_names.iter().find(|n| n.to_lowercase().contains(&query)));
                if let Some(name) = best {
                    debug!("Autocompleting '{}' to '{}'", text.trim(), name);
                    self.face_name.set_text(name);
                    self.face_name.set_position(-1);
                }
            }
            PersonSelectInput::NewPerson => {
                if let Some(face_id) = self.face_id {
                    let name = self.face_name.text().to_string();
                    let trimmed = name.trim();
                    if trimmed.is_empty() {
                        // Nothing typed: keep the selector open.
                        return;
                    }

                    // If the typed name matches an existing person, associate
                    // with them rather than creating a duplicate.
                    let existing = self
                        .people_repo
                        .all_people()
                        .unwrap_or_default()
                        .into_iter()
                        .find(|p| p.name.to_lowercase() == trimmed.to_lowercase());

                    let result = if let Some(person) = existing {
                        debug!("Face {} reuses existing person {}", face_id, person.name);
                        self.people_repo.mark_as_person(face_id, person.person_id)
                    } else {
                        debug!("Face {} is a new person '{}'", face_id, trimmed);
                        self.people_repo.add_person(face_id, trimmed)
                    };
                    if let Err(e) = result {
                        error!("Failed adding/associating person: {:?}", e);
                    }
                }
                self.people_list.remove_all();
                self.all_people.clear();
                self.face_id = None;
                let _ = sender.output(PersonSelectOutput::Done);
            }
        }
    }
}
