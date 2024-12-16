// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gio, gdk};
use relm4::*;
use relm4::prelude::*;
use relm4::actions::{RelmAction, RelmActionGroup};

use crate::fl;
use fotema_core::people;
use fotema_core::PictureId;
use fotema_core::FaceId;
use fotema_core::PersonId;

use super::person_select::{PersonSelect, PersonSelectInput, PersonSelectOutput};

use tracing::{debug, error, info};

use std::path::PathBuf;


relm4::new_action_group!(FaceActionGroup, "face");

// Associate face with a person.
relm4::new_stateless_action!(FaceSetPersonAction, FaceActionGroup, "set_person");

// Disassociate a face from a person.
relm4::new_stateless_action!(FaceNotPersonAction, FaceActionGroup, "not_person");

// Mark a face as not being a face.
relm4::new_stateless_action!(FaceIgnoreAction, FaceActionGroup, "ignore");

// Associate person with new face thumbnail.
relm4::new_stateless_action!(FaceThumbnailAction, FaceActionGroup, "thumbnail");

#[derive(Debug)]
pub enum FaceThumbnailsInput {
    /// View an item.
    View(PictureId),

    /// Reload face and person data for current picture
    Refresh,

    /// The photo/video page has been hidden so any playing media should stop.
    Hide,

    /// Associate face with a person by name.
    SetPerson(FaceId, PathBuf),

    /// Disassociate face from person.
    NotPerson(FaceId),

    /// Ignore a face.
    Ignore(FaceId),

    /// Set new thumbnail for person
    SetThumbnail(PersonId, FaceId),

    /// The person selection dialog has selected or created a new person and should be dismissed.
    PersonSelected,

}

#[derive(Debug)]
pub enum FaceThumbnailsOutput {

}

pub struct FaceThumbnails {
    people_repo: people::Repository,

    picture_id: Option<PictureId>,

    face_thumbnails: gtk::Box,

    person_dialog: adw::Dialog,
    person_select: AsyncController<PersonSelect>,
}

const AVATAR_SIZE: i32 = 50;

#[relm4::component(pub async)]
impl SimpleAsyncComponent for FaceThumbnails {
    type Init = people::Repository;
    type Input = FaceThumbnailsInput;
    type Output = FaceThumbnailsOutput;

    view! {
        #[name(face_thumbnails)]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
        }
    }

    async fn init(
        people_repo: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let widgets = view_output!();

        let person_select = PersonSelect::builder()
            .launch(people_repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                PersonSelectOutput::Done => FaceThumbnailsInput::PersonSelected,
            });

        let person_dialog = adw::Dialog::builder()
            .child(person_select.widget())
            .presentation_mode(adw::DialogPresentationMode::BottomSheet)
            .height_request(400) // FIXME make more dynamic?
            .build();

        let model = Self {
            picture_id: None,
            people_repo,
            face_thumbnails: widgets.face_thumbnails.clone(),
            person_dialog,
            person_select,

        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            FaceThumbnailsInput::Hide => {
                self.face_thumbnails.remove_all();
            },
            FaceThumbnailsInput::View(picture_id) => {
                self.picture_id = Some(picture_id);
                sender.input(FaceThumbnailsInput::Refresh);
            },
            FaceThumbnailsInput::Refresh => {

                self.face_thumbnails.remove_all();

                let Some(picture_id) = self.picture_id else {
                    return;
                };

                info!("Showing faces for {}", picture_id);

                let result = self.people_repo.find_faces(&picture_id);
                if let Err(e) = result {
                    error!("Failed getting faces: {}", e);
                    return;
                }

                if let Ok(faces) = result {
                    debug!("Found {} faces", faces.len());
                    faces.into_iter()
                        .filter(|(face, _)| face.thumbnail_path.exists())
                        .for_each(|(face, person)| {
                            let mut group = RelmActionGroup::<FaceActionGroup>::new();

                            let menu_model = gio::Menu::new();

                            let is_known_person = person.is_some();

                            let (menu_items, thumbnail_path) = if let Some(person) = person {
                                let not_person: RelmAction<FaceNotPersonAction> = {
                                    let sender = sender.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::NotPerson(face.face_id));
                                    })
                                };
                                group.add_action(not_person);

                                let set_thumbnail: RelmAction<FaceThumbnailAction> = {
                                    let sender = sender.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::SetThumbnail(person.person_id, face.face_id));
                                    })
                                };
                                group.add_action(set_thumbnail);

                                let menu_items = vec![
                                  //  gio::MenuItem::new(Some(&fl!("people-view-more-photos", name = person.name.clone())), None),
                                    gio::MenuItem::new(Some(&fl!("people-set-face-thumbnail")), Some("face.thumbnail")),
                                    gio::MenuItem::new(Some(&fl!("people-not-this-person", name = person.name.clone())), Some("face.not_person")),
                                ];

                                (menu_items, person.thumbnail_path)
                            } else {
                                let set_person: RelmAction<FaceSetPersonAction> = {
                                    let sender = sender.clone();
                                    let thumbnail_path = face.thumbnail_path.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::SetPerson(face.face_id, thumbnail_path.clone()));
                                    })
                                };
                                group.add_action(set_person);

                                let not_a_face: RelmAction<FaceIgnoreAction> = {
                                    let sender = sender.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::Ignore(face.face_id));
                                    })
                                };
                                group.add_action(not_a_face);

                                let menu_items = vec![
                                    gio::MenuItem::new(Some(&fl!("people-set-name")), Some("face.set_person")),
                                    gio::MenuItem::new(Some(&fl!("people-face-ignore")), Some("face.ignore")),
                                ];

                                (menu_items, face.thumbnail_path)
                            };

                            for item in menu_items {
                                menu_model.append_item(&item);
                            }

                            let pop = gtk::PopoverMenu::builder()
                                .menu_model(&menu_model)
                                .build();

                            let avatar = adw::Avatar::builder()
                                .size(AVATAR_SIZE)
                                .build();

                            let img = gdk::Texture::from_filename(&thumbnail_path).ok();
                            avatar.set_custom_image(img.as_ref());
                            //avatar.add_css_class(face.orientation.as_ref());

                            let children = gtk::Box::new(gtk::Orientation::Vertical, 0);
                            children.append(&avatar);
                            children.append(&pop);

                            let frame = gtk::Frame::new(None);
                            frame.add_css_class("face-small");
                            frame.set_child(Some(&children));

                            let frame = if !is_known_person {

                                let overlay = gtk::Overlay::builder()
                                    .child(&frame)
                                    .build();

                                let face_icon = gtk::Image::builder()
                                    .width_request(16)
                                    .height_request(16)
                                    .icon_name("reaction-add-symbolic")
                                    .build();

                                let label_frame = gtk::Frame::builder()
                                    .halign(gtk::Align::End)
                                    .valign(gtk::Align::End)
                                    .css_classes(["face-thumbnail-label-frame"])
                                    .child(&face_icon)
                                    .build();

                                overlay.add_overlay(&label_frame);

                                let frame = gtk::Frame::new(None);
                                frame.set_child(Some(&overlay));
                                frame.add_css_class("face-thumbnail-overlay");
                                frame
                            } else {
                                frame
                            };

                            group.register_for_widget(&frame);

                            let click = gtk::GestureClick::new();
                            click.connect_released(move |_click,_,_,_| {
                                pop.popup();
                            });

                            // if we get a stop message, then we are not dealing with a single-click.
                            click.connect_stopped(move |click| click.reset());

                            frame.add_controller(click);

                            self.face_thumbnails.append(&frame);
                        });
                }
            },
            FaceThumbnailsInput::SetPerson(face_id, thumbnail) => {
                debug!("Set person for face {}", face_id);
                if let Some(root) = gtk::Widget::root(self.face_thumbnails.widget_ref()) {

                    self.person_select.emit(PersonSelectInput::Activate(face_id, thumbnail));
                    self.person_dialog.present(Some(&root));
                } else {
                    error!("Couldn't get root widget!");
                }
                sender.input(FaceThumbnailsInput::Refresh);
            },
            FaceThumbnailsInput::SetThumbnail(person_id, face_id) => {
                debug!("Set face {} as thumbnail for person {}", face_id, person_id);
                if let Err(e) = self.people_repo.set_person_thumbnail(person_id, face_id) {
                    error!("Failed setting thumbnail: {}", e);
                }
                sender.input(FaceThumbnailsInput::Refresh);
            },
            FaceThumbnailsInput::NotPerson(face_id) => {
                debug!("Set not person for face: {}", face_id);
                if let Err(e) = self.people_repo.mark_not_person(face_id) {
                    error!("Failed marking face as not person: {}", e);
                }
                sender.input(FaceThumbnailsInput::Refresh);
            },
            FaceThumbnailsInput::Ignore(face_id) => {
                debug!("Ignoring face: {}", face_id);
                if let Err(e) = self.people_repo.mark_ignore(face_id) {
                    error!("Failed marking face as not a face: {}", e);
                }
                sender.input(FaceThumbnailsInput::Refresh);
            },
            FaceThumbnailsInput::PersonSelected => {
                debug!("Dismissing dialog.");
                self.person_dialog.close();
                sender.input(FaceThumbnailsInput::Refresh);
            },
        }
    }
}
