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
use std::cell::RefCell;
use std::path;
use std::rc::Rc;

#[derive(Debug)]
struct PhotoGridItem {
    picture: photos_core::repo::Picture,
}

struct Widgets {
    picture: gtk::Picture,
    label: gtk::Label,
}
#[derive(Debug)]
pub enum MonthPhotosInput {
    /// A month has been selected in the grid view
    MonthSelected(u32),

    /// Scroll to first photo of year
    GoToYear(i32),
}

#[derive(Debug)]
pub enum MonthPhotosOutput {
    MonthSelected(photos_core::repo::YearMonth),
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
                gtk::Label {},

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
        let ym = self.picture.year_month();

        widgets
            .label
            .set_label(format!("{} {}", ym.month().name(), ym.year()).as_str());

        if widgets.picture.file().is_none() {
            widgets
                .picture
                .set_filename(self.picture.square_preview_path.clone());
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&path::Path>);
    }
}

pub struct MonthPhotos {
    //    controller: photos_core::Controller,
    pictures_grid_view: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for MonthPhotos {
    type Init = Rc<RefCell<photos_core::Controller>>;
    type Input = MonthPhotosInput;
    type Output = MonthPhotosOutput;

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
                        sender.input(MonthPhotosInput::MonthSelected(idx))
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
            .dedup_by(|x, y| x.year_month() == y.year_month())
            .map(|picture| PhotoGridItem {
                picture,
            });

        let mut grid_view_wrapper: TypedGridView<PhotoGridItem, gtk::SingleSelection> =
            TypedGridView::new();

        grid_view_wrapper.extend_from_iter(all_pictures.into_iter());

        let model = MonthPhotos {
            pictures_grid_view: grid_view_wrapper,
        };

        let pictures_box = &model.pictures_grid_view.view;

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
           MonthPhotosInput::MonthSelected(index) => {
                if let Some(item) = self.pictures_grid_view.get(index) {
                    let ym = item.borrow().picture.year_month();
                    println!("index {} has year_month {}", index, ym);
                    let result = sender.output(MonthPhotosOutput::MonthSelected(ym));
                    println!("Result = {:?}", result);
                }
            }
           MonthPhotosInput::GoToYear(year) => {
                println!("Showing for year: {}", year);
                let index_opt = self.pictures_grid_view.find(|p| p.picture.year_month().year() == year);
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
