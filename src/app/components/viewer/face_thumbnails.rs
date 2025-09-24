// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::*;
use relm4::actions::{RelmAction, RelmActionGroup};
use relm4::adw::{self, prelude::*};
use relm4::binding::*;
use relm4::gtk::prelude::AdjustmentExt;
use relm4::gtk::prelude::*;
use relm4::gtk::{self, gdk, gio};
use relm4::prelude::*;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use gtk::prelude::OrientableExt;

use std::rc::Rc;

use crate::fl;
use fotema_core::FaceId;
use fotema_core::PersonId;
use fotema_core::PictureId;
use fotema_core::people;

use super::person_select::{PersonSelect, PersonSelectInput, PersonSelectOutput};

use tracing::{debug, error, info};

use std::path::PathBuf;


// Face thumbnails generated at this size in face_extractor.rs
const AVATAR_SIZE: i32 = 64;

relm4::new_action_group!(FaceActionGroup, "face");

// Associate face with a person.
relm4::new_stateless_action!(FaceSetPersonAction, FaceActionGroup, "set_person");

// Disassociate a face from a person.
relm4::new_stateless_action!(FaceNotPersonAction, FaceActionGroup, "not_person");

// Mark a face as not being a face.
relm4::new_stateless_action!(FaceIgnoreAction, FaceActionGroup, "ignore");

// Associate person with new face thumbnail.
relm4::new_stateless_action!(FaceThumbnailAction, FaceActionGroup, "thumbnail");

pub struct FaceGridItem {
    face: people::Face,

    /// Person for avatar
    person: Option<people::Person>,

    sender: AsyncComponentSender<FaceThumbnails>,

    menu_model: gio::Menu,
}

pub struct FaceGridItemWidgets {
    avatar: adw::Avatar,

    container: gtk::Box,

    face_icon: gtk::Frame,

    // If the avatar has been bound to edge_length.
    is_bound: bool,
}


impl RelmGridItem for FaceGridItem {
    type Root = gtk::Frame;
    type Widgets = FaceGridItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {

        relm4::view! {
            root = gtk::Frame {
                    add_css_class: "face-thumbnail-overlay",
                    gtk::Overlay {

                        #[name(face_icon)]
                        add_overlay = &gtk::Frame {
                            set_valign: gtk::Align::End,
                            set_halign: gtk::Align::End,
                            add_css_class: "face-thumbnail-label-frame",

                            #[wrap(Some)]
                            set_child = &gtk::Image {
                                set_width_request: 16,
                                set_height_request: 16,

                                //#[wrap(Some)]
                                set_icon_name: Some("reaction-add-symbolic"),
                            },
                        },

                        #[wrap(Some)]
                        set_child = &gtk::Frame {
                            add_css_class: "face-small",
                            #[name(container)]
                            gtk::Box {
                                set_orientation: gtk::Orientation::Vertical,

                                #[name(avatar)]
                                adw::Avatar {
                                    set_size: AVATAR_SIZE,
                                },
                            }
                        }
                    }
                }
            }

        let widgets = FaceGridItemWidgets {
            avatar,
            container,
            face_icon,
            is_bound: false,
        };

        (root, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root) {

        let mut group = RelmActionGroup::<FaceActionGroup>::new();

        let menu_model = gio::Menu::new();

        // Only show face icon on unknown faces
        widgets.face_icon.set_visible(self.person.is_some());

        let (menu_items, thumbnail_path) = if let Some(person) = self.person.as_ref() {
            let not_person: RelmAction<FaceNotPersonAction> = {
                let sender = self.sender.clone();
                let face_id = self.face.face_id;
                RelmAction::new_stateless(move |_| {
                    sender.input(FaceThumbnailsInput::NotPerson(face_id));
                })
            };
            group.add_action(not_person);

            let set_thumbnail: RelmAction<FaceThumbnailAction> = {
                let sender = self.sender.clone();
                let person_id = person.person_id;
                let face_id = self.face.face_id;
                RelmAction::new_stateless(move |_| {
                    sender.input(FaceThumbnailsInput::SetThumbnail(
                        person_id,
                        face_id,
                    ));
                })
            };
            group.add_action(set_thumbnail);

            let menu_items = vec![
                //  gio::MenuItem::new(Some(&fl!("people-view-more-photos", name = person.name.clone())), None),
                gio::MenuItem::new(
                    Some(&fl!("people-set-face-thumbnail")),
                    Some("face.thumbnail"),
                ),
                gio::MenuItem::new(
                    Some(&fl!(
                        "people-not-this-person",
                        name = person.name.clone()
                    )),
                    Some("face.not_person"),
                ),
            ];

            (menu_items, person.small_thumbnail_path.clone())
        } else {
            let set_person: RelmAction<FaceSetPersonAction> = {
                let sender = self.sender.clone();
                let face_id = self.face.face_id;
                let thumbnail_path = self.face.thumbnail_path.clone();
                RelmAction::new_stateless(move |_| {
                    sender.input(FaceThumbnailsInput::SetPerson(
                        face_id,
                        thumbnail_path.clone(),
                    ));
                })
            };
            group.add_action(set_person);

            let not_a_face: RelmAction<FaceIgnoreAction> = {
                let sender = self.sender.clone();
                let face_id = self.face.face_id;
                RelmAction::new_stateless(move |_| {
                    sender.input(FaceThumbnailsInput::Ignore(face_id));
                })
            };
            group.add_action(not_a_face);

            let menu_items = vec![
                gio::MenuItem::new(
                    Some(&fl!("people-set-name")),
                    Some("face.set_person"),
                ),
                gio::MenuItem::new(
                    Some(&fl!("people-face-ignore")),
                    Some("face.ignore"),
                ),
            ];

            (menu_items, Some(self.face.thumbnail_path.clone()))
        };

        for item in menu_items {
            menu_model.append_item(&item);
        }

        let pop = gtk::PopoverMenu::builder().menu_model(&menu_model).build();
        pop.set_menu_model(Some(&menu_model));

        widgets.container.append(&pop);

        group.register_for_widget(&root);

        let click = gtk::GestureClick::new();
        click.connect_released(move |_click, _, _, _| {
            pop.popup();
        });

        // if we get a stop message, then we are not dealing with a single-click.
        click.connect_stopped(move |click| click.reset());

        widgets.avatar.add_controller(click);

        let thumbnail_path = self.person.as_ref()
            .and_then(|p| p.small_thumbnail_path.as_ref())
            .unwrap_or_else( || &self.face.thumbnail_path);

        let img = gdk::Texture::from_filename(thumbnail_path).ok();
        widgets.avatar.set_custom_image(img.as_ref());
    }
}

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
pub enum FaceThumbnailsOutput {}

pub struct FaceThumbnails {
    people_repo: people::Repository,

    picture_id: Option<PictureId>,

    face_thumbnails: gtk::Box,
    face_grid: TypedGridView<FaceGridItem, gtk::SingleSelection>,
    faces_and_people: Vec<(people::FaceId, Option<people::PersonId>)>,

    person_dialog: adw::Dialog,
    person_select: AsyncController<PersonSelect>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for FaceThumbnails {
    type Init = people::Repository;
    type Input = FaceThumbnailsInput;
    type Output = FaceThumbnailsOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            gtk::ScrolledWindow {
                set_hexpand: true,
                set_vscrollbar_policy: gtk::PolicyType::Never,
                set_hscrollbar_policy: gtk::PolicyType::External, // scroll bar not visible, but faces scrollable
                set_propagate_natural_width: true,

                #[name(face_thumbnails)]
                gtk::Box {
                    set_hexpand: false,
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,
                }
            },

            #[local_ref]
            grid_view -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,
            },
        }
    }

    async fn init(
        people_repo: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {

        let face_grid: TypedGridView<FaceGridItem, gtk::SingleSelection>  = TypedGridView::new();
        let grid_view = &face_grid.view.clone();

        let widgets = view_output!();

        let person_select = PersonSelect::builder().launch(people_repo.clone()).forward(
            sender.input_sender(),
            |msg| match msg {
                PersonSelectOutput::Done => FaceThumbnailsInput::PersonSelected,
            },
        );

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
            face_grid,
            faces_and_people: Vec::new(),
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            FaceThumbnailsInput::Hide => {
                self.face_thumbnails.remove_all();
            }
            FaceThumbnailsInput::View(picture_id) => {
                self.picture_id = Some(picture_id);
                sender.input(FaceThumbnailsInput::Refresh);
            }
            FaceThumbnailsInput::Refresh => {
                self.face_thumbnails.remove_all();
                self.face_grid.clear();
                self.faces_and_people.clear();

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
                    faces
                        .into_iter()
                        .filter(|(face, _)| face.thumbnail_path.exists())
                        .for_each(|(face, person)| {

                            let mut group = RelmActionGroup::<FaceActionGroup>::new();

                            let menu_model = gio::Menu::new();

                            let is_known_person = person.is_some();

                            let (menu_items, thumbnail_path) = if let Some(ref person) = person {
                                let face = face.clone();
                                let not_person: RelmAction<FaceNotPersonAction> = {
                                    let sender = sender.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::NotPerson(face.face_id));
                                    })
                                };
                                group.add_action(not_person);

                                let set_thumbnail: RelmAction<FaceThumbnailAction> = {
                                    let sender = sender.clone();
                                    let person_id = person.person_id;
                                    let face_id = face.face_id;
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::SetThumbnail(
                                            person_id,
                                            face_id,
                                        ));
                                    })
                                };
                                group.add_action(set_thumbnail);

                                let menu_items = vec![
                                    //  gio::MenuItem::new(Some(&fl!("people-view-more-photos", name = person.name.clone())), None),
                                    gio::MenuItem::new(
                                        Some(&fl!("people-set-face-thumbnail")),
                                        Some("face.thumbnail"),
                                    ),
                                    gio::MenuItem::new(
                                        Some(&fl!(
                                            "people-not-this-person",
                                            name = person.name.clone()
                                        )),
                                        Some("face.not_person"),
                                    ),
                                ];

                                (menu_items, person.small_thumbnail_path.clone())
                            } else {
                                let face = face.clone();
                                let set_person: RelmAction<FaceSetPersonAction> = {
                                    let sender = sender.clone();
                                    let thumbnail_path = face.thumbnail_path.clone();
                                    RelmAction::new_stateless(move |_| {
                                        sender.input(FaceThumbnailsInput::SetPerson(
                                            face.face_id,
                                            thumbnail_path.clone(),
                                        ));
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
                                    gio::MenuItem::new(
                                        Some(&fl!("people-set-name")),
                                        Some("face.set_person"),
                                    ),
                                    gio::MenuItem::new(
                                        Some(&fl!("people-face-ignore")),
                                        Some("face.ignore"),
                                    ),
                                ];

                                (menu_items, Some(face.thumbnail_path))
                            };

                            for item in menu_items {
                                menu_model.append_item(&item);
                            }

                            let item = FaceGridItem {
                                face: face.clone(),
                                person: person,
                                menu_model: menu_model.clone(),
                                sender: sender.clone(),
                            };

                            self.face_grid.append(item);

                            let pop = gtk::PopoverMenu::builder().menu_model(&menu_model).build();

                            let avatar = adw::Avatar::builder().size(AVATAR_SIZE).build();

                            if let Some(thumbnail_path) = thumbnail_path {
                                let img = gdk::Texture::from_filename(&thumbnail_path).ok();
                                avatar.set_custom_image(img.as_ref());
                            }

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
                            click.connect_released(move |_click, _, _, _| {
                                pop.popup();
                            });

                            // if we get a stop message, then we are not dealing with a single-click.
                            click.connect_stopped(move |click| click.reset());

                            frame.add_controller(click);

                            self.face_thumbnails.append(&frame);
                        });
                }
            }
            FaceThumbnailsInput::SetPerson(face_id, thumbnail) => {
                debug!("Set person for face {}", face_id);
                if let Some(root) = gtk::Widget::root(self.face_thumbnails.widget_ref()) {
                    self.person_select
                        .emit(PersonSelectInput::Activate(face_id, thumbnail));
                    self.person_dialog.present(Some(&root));
                } else {
                    error!("Couldn't get root widget!");
                }
                sender.input(FaceThumbnailsInput::Refresh);
            }
            FaceThumbnailsInput::SetThumbnail(person_id, face_id) => {
                debug!("Set face {} as thumbnail for person {}", face_id, person_id);
                if let Err(e) = self.people_repo.set_person_thumbnail(person_id, face_id) {
                    error!("Failed setting thumbnail: {}", e);
                }
                sender.input(FaceThumbnailsInput::Refresh);
            }
            FaceThumbnailsInput::NotPerson(face_id) => {
                debug!("Set not person for face: {}", face_id);
                if let Err(e) = self.people_repo.mark_not_person(face_id) {
                    error!("Failed marking face as not person: {}", e);
                }
                sender.input(FaceThumbnailsInput::Refresh);
            }
            FaceThumbnailsInput::Ignore(face_id) => {
                debug!("Ignoring face: {}", face_id);
                if let Err(e) = self.people_repo.mark_ignore(face_id) {
                    error!("Failed marking face as not a face: {}", e);
                }
                sender.input(FaceThumbnailsInput::Refresh);
            }
            FaceThumbnailsInput::PersonSelected => {
                debug!("Dismissing dialog.");
                self.person_dialog.close();
                sender.input(FaceThumbnailsInput::Refresh);
            }
        }
    }
}
