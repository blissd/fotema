// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::{BoxExt, OrientableExt};
use photos_core;
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use std::path;
use std::sync::{Arc, Mutex};
use photos_core::YearMonth;

#[derive(Debug)]
struct PhotoGridItem {
    picture: photos_core::repo::Picture,
}

struct Widgets {
    picture: gtk::Picture,
}

#[derive(Debug)]
pub enum AllPhotosInput {
    /// User has selected photo in grid view
    PhotoSelected(u32), // Index into a Vec

    // Scroll to first photo of year/month.
    GoToMonth(YearMonth),
}

#[derive(Debug)]
pub enum AllPhotosOutput {
    /// User has selected photo in grid view
    PhotoSelected(photos_core::repo::PictureId),
}

impl RelmGridItem for PhotoGridItem {
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
        if self.picture.square_preview_path.as_ref().is_some_and(|f|f.exists()) {
            widgets
                .picture
                .set_filename(self.picture.square_preview_path.clone());
        } else {
            widgets
                .picture
                .set_resource(Some("/dev/romantics/Photos/icons/image-missing-symbolic.svg"));
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&path::Path>);
    }
}

#[derive(Debug)]
pub struct AllPhotos {
    //    controller: photos_core::Controller,
    pictures_grid_view: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for AllPhotos {
    type Init = Arc<Mutex<photos_core::Repository>>;
    type Input = AllPhotosInput;
    type Output = AllPhotosOutput;

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
                        sender.input(AllPhotosInput::PhotoSelected(idx))
                    },
                },
            },
        }
    }

    fn init(
        repo: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let all_pictures = repo
            .lock()
            .unwrap()
            .all()
            .unwrap()
            .into_iter()
            .map(|picture| PhotoGridItem {
                picture,
            });

        let mut grid_view_wrapper: TypedGridView<PhotoGridItem, gtk::SingleSelection> =
            TypedGridView::new();

        grid_view_wrapper.extend_from_iter(all_pictures.into_iter());

        let model = AllPhotos {
            pictures_grid_view: grid_view_wrapper,
        };

        let pictures_box = &model.pictures_grid_view.view;

        pictures_box.scroll_to(model.pictures_grid_view.len(), gtk::ListScrollFlags::SELECT, None);

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AllPhotosInput::PhotoSelected(index) => {
                if let Some(item) = self.pictures_grid_view.get(index) {
                    let picture_id = item.borrow().picture.picture_id;
                    println!("index {} has picture_id {}", index, picture_id);
                    let result = sender.output(AllPhotosOutput::PhotoSelected(picture_id));
                    println!("Result = {:?}", result);
                }
            },
            AllPhotosInput::GoToMonth(ym) => {
                println!("Showing for month: {}", ym);
                let index_opt = self.pictures_grid_view.find(|p| p.picture.year_month() == ym);
                println!("Found: {:?}", index_opt);
                if let Some(index) = index_opt {
                    let flags = gtk::ListScrollFlags::SELECT;
                    println!("Scrolling to {}", index);
                    self.pictures_grid_view.view.scroll_to(index, flags, None);
                }
            }
        }
    }
}
