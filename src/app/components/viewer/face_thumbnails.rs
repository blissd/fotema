// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gio};
use relm4::*;
use relm4::prelude::*;
use relm4::actions::{RelmAction, RelmActionGroup};

use crate::fl;
use fotema_core::people;
use fotema_core::PictureId;
use fotema_core::FaceId;

use tracing::{debug, error, info};

use std::path::PathBuf;


relm4::new_action_group!(FaceActionGroup, "face");

// Associate face with a person.
relm4::new_stateless_action!(FaceSetPersonAction, FaceActionGroup, "set_person");

// Disassociate a face from a person.
relm4::new_stateless_action!(FaceNotPersonAction, FaceActionGroup, "not_person");

// Mark a face as not being a face.
relm4::new_stateless_action!(FaceNotFaceAction, FaceActionGroup, "not_a_face");


#[derive(Debug)]
pub enum FaceThumbnailsInput {
    // View an item.
    View(PictureId),

    // The photo/video page has been hidden so any playing media should stop.
    Hide,

    /// Associate face with a person by name.
    SetPerson(FaceId, PathBuf),

    /// Disassociate face from person.
    NotPerson(FaceId),

    /// Mark that a face isn't actually a face.
    NotFace(FaceId),

}

#[derive(Debug)]
pub enum FaceThumbnailsOutput {

}

pub struct FaceThumbnails {
    people_repo: people::Repository,

    face_thumbnails: gtk::Box,

    person_dialog: adw::Dialog,
    person_dialog_thumbnail: gtk::Picture,
}

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
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let widgets = view_output!();

        let person_dialog_thumbnail = gtk::Picture::builder()
            .width_request(100)
            .height_request(100)
            .content_fit(gtk::ContentFit::Fill)
            .build();
        person_dialog_thumbnail.add_css_class("face-small");

        let dialog_items = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .build();

        dialog_items.append(&person_dialog_thumbnail);

        let person_dialog = adw::Dialog::builder()
            .child(&dialog_items)
            .build();

        let model = Self {
            people_repo,
            face_thumbnails: widgets.face_thumbnails.clone(),
            person_dialog,
            person_dialog_thumbnail,

        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            FaceThumbnailsInput::Hide => {
                self.face_thumbnails.remove_all();
            },
            FaceThumbnailsInput::View(picture_id) => {
                info!("Showing faces for {}", picture_id);

                self.face_thumbnails.remove_all();

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

                            let menu_items = if let Some(person) = person {
                                let not_person: RelmAction<FaceNotPersonAction> = {
                                    let sender = sender.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::NotPerson(face.face_id));
                                    })
                                };
                                group.add_action(not_person);

                                vec![
                                    gio::MenuItem::new(Some(&fl!("people-view-more-photos", name = person.name)), None),
                                    gio::MenuItem::new(Some(&fl!("people-set-key-face")), Some("face.not_person")),
                                    gio::MenuItem::new(Some(&fl!("people-not-this-person")), None),
                                ]
                            } else {
                                let set_person: RelmAction<FaceSetPersonAction> = {
                                    let sender = sender.clone();
                                    let thumbnail_path = face.thumbnail_path.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::SetPerson(face.face_id, thumbnail_path.clone()));
                                    })
                                };
                                group.add_action(set_person);

                                let not_a_face: RelmAction<FaceNotFaceAction> = {
                                    let sender = sender.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::NotFace(face.face_id));
                                    })
                                };
                                group.add_action(not_a_face);

                                vec![
                                    gio::MenuItem::new(Some(&fl!("people-set-name")), Some("face.set_person")),
                                    gio::MenuItem::new(Some(&fl!("people-not-a-face")), Some("face.not_a_face")),
                                ]
                            };

                            for item in menu_items {
                                menu_model.append_item(&item);
                            }

                            let pop = gtk::PopoverMenu::builder()
                                .menu_model(&menu_model)
                                .build();

                            let thumbnail = gtk::Picture::for_filename(&face.thumbnail_path);
                            thumbnail.set_content_fit(gtk::ContentFit::ScaleDown);
                            thumbnail.set_width_request(50);
                            thumbnail.set_height_request(50);

                            let children = gtk::Box::new(gtk::Orientation::Vertical, 0);
                            children.append(&thumbnail);
                            children.append(&pop);

                            let frame = gtk::Frame::new(None);
                            frame.add_css_class("face-small");
                            frame.set_child(Some(&children));

                            let frame = if !is_known_person {

                                let overlay = gtk::Overlay::builder()
                                    .child(&frame)
                                    .build();

                                let label = gtk::Label::builder()
                                    .label("?")
                                    .css_classes(["face-thumbnail-label-text"])
                                    .build();

                                let label_frame = gtk::Frame::builder()
                                    .halign(gtk::Align::End)
                                    .valign(gtk::Align::End)
                                    .css_classes(["face-thumbnail-label-frame"])
                                    .child(&label)
                                    .build();

                                //label_frame.set_margin_all(4);

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
                println!("set person for face {}", face_id);
                if let Some(root) = gtk::Widget::root(self.face_thumbnails.widget_ref()) {
                    self.person_dialog_thumbnail.set_filename(Some(thumbnail));
                    self.person_dialog.present(&root);
                } else {
                    error!("Couldn't get root widget!");
                }
            },
            FaceThumbnailsInput::NotPerson(face_id) => {
                println!("set not person for face {}", face_id);
            },
            FaceThumbnailsInput::NotFace(face_id) => {
                println!("not a face! {}", face_id);
                if let Err(e) = self.people_repo.mark_not_a_face(face_id) {
                    error!("Failed marking face as not a face: {}", e);
                }
            },
        }
    }
}
