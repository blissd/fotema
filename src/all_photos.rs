// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::{BoxExt, OrientableExt};
use photos_core;
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use std::cell::RefCell;
use std::path;
use std::rc::Rc;

#[derive(Debug)]
pub struct PicturePreview {
    controller: Rc<RefCell<photos_core::Controller>>,
    picture: photos_core::repo::Picture,
}

pub struct Widgets {
    picture: gtk::Picture,
}

#[derive(Debug)]
pub enum PhotoGridInput {
    /// View picture at given offset of gridview
    ViewPhoto(u32),
}

#[derive(Debug)]
pub enum PhotoGridOutput {
    ViewPhoto(photos_core::repo::PictureId),
}

impl RelmGridItem for PicturePreview {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
           my_box = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 1,

                #[name = "picture"]
                gtk::Picture {
                    set_can_shrink: true,
                    set_valign: gtk::Align::Center,
                    set_width_request: 200,
                    set_height_request: 200,
                }
            }
        }

        let widgets = Widgets { picture };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets
            .picture
            .set_filename(self.picture.square_preview_path.clone());
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&path::Path>);
    }
}

#[derive(Debug)]
pub struct AllPhotos {
    //    controller: photos_core::Controller,
    pictures_grid_view: TypedGridView<PicturePreview, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for AllPhotos {
    type Init = Rc<RefCell<photos_core::Controller>>;
    type Input = PhotoGridInput;
    type Output = PhotoGridOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 0,
            set_margin_all: 0,

            gtk::ScrolledWindow {

                //set_propagate_natural_height: true,
                //set_has_frame: true,
                set_vexpand: true,

                #[local_ref]
                pictures_box -> gtk::GridView {
                    set_orientation: gtk::Orientation::Vertical,
                    set_single_click_activate: true,
                    //set_max_columns: 3,

                    connect_activate[sender] => move |_, idx| {
                        sender.input(PhotoGridInput::ViewPhoto(idx))
                    },
                },
            },
        }
    }

    fn init(
        controller: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let all_pictures = controller
            .borrow_mut()
            //.all()
            .all_with_previews()
            .unwrap()
            .into_iter()
            .map(|picture| PicturePreview {
                picture,
                controller: controller.clone(),
            });

        let mut grid_view_wrapper: TypedGridView<PicturePreview, gtk::SingleSelection> =
            TypedGridView::new();

        grid_view_wrapper.extend_from_iter(all_pictures.into_iter());

        let model = AllPhotos {
            pictures_grid_view: grid_view_wrapper,
        };

        let pictures_box = &model.pictures_grid_view.view;

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PhotoGridInput::ViewPhoto(index) => {
                if let Some(item) = self.pictures_grid_view.get(index) {
                    let picture_id = item.borrow().picture.picture_id;
                    println!("index {} has picture_id {}", index, picture_id);
                    sender
                        .output(PhotoGridOutput::ViewPhoto(picture_id))
                        .unwrap();
                }
            }
        }
    }
}
