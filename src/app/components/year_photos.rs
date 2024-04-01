// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::{BoxExt, OrientableExt};
use photos_core;

use itertools::Itertools;
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use std::path;
use std::sync::{Arc, Mutex};
use photos_core::Year;

#[derive(Debug)]
struct PhotoGridItem {
    picture: photos_core::repo::Picture,
}
#[derive(Debug)]
pub enum YearPhotosInput {
    /// User has selected year in grid view
    YearSelected(u32), // WARN this is an index into an Vec, not a year.
}

#[derive(Debug)]
pub enum YearPhotosOutput {
    YearSelected(Year),
}

struct Widgets {
    picture: gtk::Picture,
    label: gtk::Label,
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
           my_box = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 1,

                #[name(label)]
                gtk::Label {
                    add_css_class: "caption-heading",
                },

                adw::Clamp {
                    set_maximum_size: 200,

                    gtk::Frame {

                        #[name(picture)]
                        gtk::Picture {
                            set_can_shrink: true,
                            set_valign: gtk::Align::Center,
                            set_width_request: 200,
                            set_height_request: 200,
                        }
                    }
                }
            }
        }

        let widgets = Widgets { picture, label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets
            .label
            .set_label(format!("{}", self.picture.year()).as_str());

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

pub struct YearPhotos {
    pictures_grid_view: TypedGridView<PhotoGridItem, gtk::NoSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for YearPhotos {
    type Init = Arc<Mutex<photos_core::Repository>>;
    type Input = YearPhotosInput;
    type Output = YearPhotosOutput;

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
                        sender.input(YearPhotosInput::YearSelected(idx))
                    },
                },
            },
        }
    }

    fn init(
        repo: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let all_pictures = repo
            .lock().unwrap()
            .all()
            .unwrap()
            .into_iter()
            .dedup_by(|x, y| x.year() == y.year())
            .map(|picture| PhotoGridItem {
                picture,
            });

        let mut grid_view_wrapper: TypedGridView<PhotoGridItem, gtk::NoSelection> =
            TypedGridView::new();

        grid_view_wrapper.extend_from_iter(all_pictures.into_iter());

        let model = YearPhotos {
            pictures_grid_view: grid_view_wrapper,
        };

        let pictures_box = &model.pictures_grid_view.view;
        if !model.pictures_grid_view.is_empty(){
            pictures_box.scroll_to(model.pictures_grid_view.len() - 1, gtk::ListScrollFlags::SELECT, None);
        }

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
           YearPhotosInput::YearSelected(index) => {
                if let Some(item) = self.pictures_grid_view.get(index) {
                    let date = item.borrow().picture.year_month();
                    println!("index {} has year {}", index, date.year);
                    let result = sender.output(YearPhotosOutput::YearSelected(date.year));
                    println!("Result = {:?}", result);
                }
            }
        }
    }
}
