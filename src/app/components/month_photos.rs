// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use fotema_core;

use itertools::Itertools;
use relm4::gtk;
use relm4::gtk::prelude::FrameExt;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;

use fotema_core::Year;
use fotema_core::YearMonth;
use std::path;
use std::sync::Arc;

#[derive(Debug)]
struct PhotoGridItem {
    picture: Arc<fotema_core::visual::repo::Visual>,
}

struct Widgets {
    picture: gtk::Picture,
    label: gtk::Label,
}
#[derive(Debug)]
pub enum MonthPhotosInput {
    /// A month has been selected in the grid view
    MonthSelected(u32), // WARN this is an index into a Vec, not a month

    /// Scroll to first photo of year
    GoToYear(Year),

    // Reload photos from database
    Refresh,
}

#[derive(Debug)]
pub enum MonthPhotosOutput {
    MonthSelected(YearMonth),
}

impl RelmGridItem for PhotoGridItem {
    type Root = adw::Clamp;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
           root = adw::Clamp {
                set_maximum_size: 200,
                gtk::Overlay {
                    add_overlay =  &gtk::Frame {
                        set_halign: gtk::Align::Start,
                        set_valign: gtk::Align::Start,
                        set_margin_start: 8,
                        set_margin_top: 8,
                        add_css_class: "photo-grid-month-frame",

                        #[wrap(Some)]
                        #[name(label)]
                        set_child = &gtk::Label{
                            add_css_class: "photo-grid-month-label",
                        },
                    },

                    #[wrap(Some)]
                    set_child = &gtk::Frame {
                        set_width_request: 200,
                        set_height_request: 200,

                        #[name(picture)]
                        gtk::Picture {
                            set_can_shrink: true,
                            set_valign: gtk::Align::Center,
                        }
                    }
                }
            }
        }

        let widgets = Widgets { picture, label };

        (root, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        let ym = self.picture.year_month();

        widgets
            .label
            .set_label(format!("{} {}", ym.month.name(), ym.year).as_str());

        if self.picture.thumbnail_path.exists() {
            widgets
                .picture
                .set_filename(Some(self.picture.thumbnail_path.clone()));
        } else {
            widgets.picture.set_resource(Some(
                "/dev/romantics/Fotema/icons/image-missing-symbolic.svg",
            ));
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&path::Path>);
    }
}

pub struct MonthPhotos {
    repo: fotema_core::visual::Library,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for MonthPhotos {
    type Init = fotema_core::visual::Library;
    type Input = MonthPhotosInput;
    type Output = MonthPhotosOutput;

    view! {
        gtk::ScrolledWindow {
            //set_propagate_natural_height: true,
            //set_has_frame: true,
            set_vexpand: true,

            #[local_ref]
            photo_grid_view -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,
                //set_max_columns: 3,

                connect_activate[sender] => move |_, idx| {
                    sender.input(MonthPhotosInput::MonthSelected(idx))
                },
            },
        }
    }

    fn init(
        repo: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let model = MonthPhotos { repo, photo_grid };

        let photo_grid_view = &model.photo_grid.view;

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            MonthPhotosInput::Refresh => {
                let all_pictures = self
                    .repo
                    .all()
                    .into_iter()
                    .filter(|x| x.thumbnail_path.exists())
                    .dedup_by(|x, y| x.year_month() == y.year_month())
                    .map(|picture| PhotoGridItem { picture });

                self.photo_grid.clear();
                self.photo_grid.extend_from_iter(all_pictures.into_iter());

                if !self.photo_grid.is_empty() {
                    self.photo_grid.view.scroll_to(
                        self.photo_grid.len() - 1,
                        gtk::ListScrollFlags::SELECT,
                        None,
                    );
                }
            }
            MonthPhotosInput::MonthSelected(index) => {
                if let Some(item) = self.photo_grid.get(index) {
                    let ym = item.borrow().picture.year_month();
                    println!("index {} has year_month {}", index, ym);
                    let result = sender.output(MonthPhotosOutput::MonthSelected(ym));
                    println!("Result = {:?}", result);
                }
            }
            MonthPhotosInput::GoToYear(year) => {
                println!("Showing for year: {}", year);
                let index_opt = self
                    .photo_grid
                    .find(|p| p.picture.year_month().year == year);
                println!("Found: {:?}", index_opt);
                if let Some(index) = index_opt {
                    let flags = gtk::ListScrollFlags::SELECT;
                    println!("Scrolling to {}", index);
                    self.photo_grid.view.scroll_to(index, flags, None);
                }
            }
        }
    }
}
