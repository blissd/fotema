// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::glib;
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
        println!(
            "binding for {:?} {:?}",
            self.picture.picture_id, self.picture.path
        );

        // compute preview image if it is absent
        if self.picture.square_preview_path.is_none() {
            let mut controller = self.controller.borrow_mut();
            let path = controller.add_preview(&mut self.picture);
            if let Ok(p) = path {
                self.picture.square_preview_path = Some(p);
            }
        }

        if widgets.picture.file().is_none() {
            widgets
                .picture
                .set_filename(self.picture.square_preview_path.clone());
        }
    }

    //fn unbind(&mut self, widgets: &mut Self::Widgets, root: &mut Self::Root) {
    //    widgets.picture.set_filename(None::<&path::Path>);
    //}
}

pub struct AllPhotos {
    //    controller: photos_core::Controller,
    pictures_grid_view: TypedGridView<PicturePreview, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for AllPhotos {
    type Init = ();
    type Input = ();
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 5,
            set_margin_all: 5,

            gtk::ScrolledWindow {
                //set_propagate_natural_height: true,
                //set_has_frame: true,
                set_vexpand: true,

                #[local_ref]
                pictures_box -> gtk::GridView {
                    set_orientation: gtk::Orientation::Vertical,
                    set_max_columns: 3,
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let data_dir = glib::user_data_dir().join("photo-romantic");
        let _ = std::fs::create_dir_all(&data_dir);

        let cache_dir = glib::user_cache_dir().join("photo-romantic");
        let _ = std::fs::create_dir_all(&cache_dir);

        let pic_base_dir = path::Path::new("/var/home/david/Pictures");
        let repo = {
            let db_path = data_dir.join("pictures.sqlite");
            photos_core::Repository::open(&pic_base_dir, &db_path).unwrap()
        };

        let scan = photos_core::Scanner::build(&pic_base_dir).unwrap();

        let previewer = {
            let preview_base_path = cache_dir.join("previews");
            let _ = std::fs::create_dir_all(&preview_base_path);
            photos_core::Previewer::build(&preview_base_path).unwrap()
        };

        let mut controller = photos_core::Controller::new(scan, repo, previewer);

        // Time consuming!
        match controller.scan() {
            Err(e) => {
                println!("Failed scanning: {:?}", e);
            }
            _ => {}
        }

        let controller = Rc::new(RefCell::new(controller));

        let all_pictures = controller
            .borrow_mut()
            .all()
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

    fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {}
}
