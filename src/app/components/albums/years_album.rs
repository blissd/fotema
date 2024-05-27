// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use fotema_core;

use fotema_core::visual::model::PictureOrientation;
use strum::IntoEnumIterator;

use itertools::Itertools;
use fotema_core::Year;

use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::gdk_pixbuf;
use relm4::gtk::prelude::FrameExt;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;

use std::path;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;

use crate::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;

const NARROW_EDGE_LENGTH: i32 = 170;
const WIDE_EDGE_LENGTH: i32 = 200;

#[derive(Debug)]
struct PhotoGridItem {
    picture: Arc<fotema_core::visual::Visual>,

    // Set of all thumbnails to allow for easy resizing on layout change.
    thumbnails: Rc<RefCell<HashSet<gtk::Picture>>>,
}
#[derive(Debug)]
pub enum YearsAlbumInput {
    Activate,

    /// User has selected year in grid view
    Selected(u32), // WARN this is an index into an Vec, not a year.

    // Reload photos from database
    Refresh,

    // Adapt to layout
    Adapt(adaptive::Layout),
}

#[derive(Debug)]
pub enum YearsAlbumOutput {
    YearSelected(Year),
}

struct Widgets {
    picture: gtk::Picture,
    label: gtk::Label,
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::AspectFrame;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            root = gtk::AspectFrame {
                gtk::Frame {
                    gtk::Overlay {
                        add_overlay =  &gtk::Frame {
                            set_halign: gtk::Align::Start,
                            set_valign: gtk::Align::Start,
                            set_margin_start: 8,
                            set_margin_top: 8,
                            add_css_class: "photo-grid-year-frame",

                            #[wrap(Some)]
                            #[name(label)]
                            set_child = &gtk::Label{
                                add_css_class: "photo-grid-year-label",
                            },
                        },

                        #[wrap(Some)]
                        #[name(picture)]
                        set_child = &gtk::Picture {
                            set_can_shrink: true,
                            set_width_request: NARROW_EDGE_LENGTH,
                            set_height_request: NARROW_EDGE_LENGTH,
                        }
                    }
                }
            }
        }

        let widgets = Widgets { picture, label };

        (root, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets
            .label
            .set_text(format!("{}", self.picture.year()).as_str());

        // Add our picture to the set of all pictures so it can be easily resized
        // when the window dimensions changes between wide and narrow.
        if !self.thumbnails.borrow().contains(&widgets.picture) {
            self.thumbnails.borrow_mut().insert(widgets.picture.clone());
        }

        if self.picture.thumbnail_path.as_ref().is_some_and(|x| x.exists()) {
            widgets
                .picture
                .set_filename(self.picture.thumbnail_path.clone());

            // Add CSS class for orientation
            let orientation = self.picture.thumbnail_orientation();
            widgets.picture.add_css_class(orientation.as_ref());
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/app/fotema/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                200, 200, true
            ).unwrap();
            let img = gdk::Texture::for_pixbuf(&pb);
            widgets.picture.set_paintable(Some(&img));
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&path::Path>);
        // clear orientation transformation css classes
        for orient in PictureOrientation::iter() {
            widgets.picture.remove_css_class(orient.as_ref());
        }
    }
}

pub struct YearsAlbum {
    state: SharedState,
    active_view: ActiveView,
    photo_grid: TypedGridView<PhotoGridItem, gtk::NoSelection>,
    layout: adaptive::Layout,
    thumbnails: Rc<RefCell<HashSet<gtk::Picture>>>,
}

pub struct YearsAlbumWidgets {
    // All pictures referenced by grid view.
    thumbnails: Rc<RefCell<HashSet<gtk::Picture>>>,
}

//#[relm4::component(pub)]
impl SimpleComponent for YearsAlbum {
    type Init = (SharedState, ActiveView);
    type Input = YearsAlbumInput;
    type Output = YearsAlbumOutput;
    type Root = gtk::ScrolledWindow;
    type Widgets = YearsAlbumWidgets;

    fn init_root() -> Self::Root {
        gtk::ScrolledWindow::builder()
            .vexpand(true)
            .build()
    }

    fn init(
        (state, active_view): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let grid_view = &photo_grid.view;
        grid_view.set_orientation(gtk::Orientation::Vertical);
        grid_view.set_single_click_activate(true);
        grid_view.connect_activate(move |_, idx| sender.input(YearsAlbumInput::Selected(idx)));

        let model = YearsAlbum {
            state,
            active_view,
            photo_grid,
            layout: adaptive::Layout::Narrow,
            thumbnails: Rc::new(RefCell::new(HashSet::new())),
        };

        let widgets = YearsAlbumWidgets {
            thumbnails: model.thumbnails.clone(),
        };

        root.set_child(Some(&model.photo_grid.view));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            YearsAlbumInput::Activate => {
                *self.active_view.write() = ViewName::Year;
                if self.photo_grid.is_empty() {
                    self.refresh();
                }
            }
            YearsAlbumInput::Refresh => {
                if *self.active_view.read() == ViewName::Year {
                    self.refresh();
                } else {
                    self.photo_grid.clear();
                }
            }
            YearsAlbumInput::Selected(index) => {
                if let Some(item) = self.photo_grid.get(index) {
                    let date = item.borrow().picture.year_month();
                    let _ = sender.output(YearsAlbumOutput::YearSelected(date.year));
                }
            },
            YearsAlbumInput::Adapt(layout) => {
                self.layout = layout;
            },
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        match self.layout {
            // Update thumbnail size depending on adaptive layout type
            adaptive::Layout::Narrow => {
                let pics = widgets.thumbnails.borrow_mut();
                for pic in pics.iter() {
                    pic.set_width_request(NARROW_EDGE_LENGTH);
                    pic.set_height_request(NARROW_EDGE_LENGTH);
                }
            },
            adaptive::Layout::Wide => {
                let pics = widgets.thumbnails.borrow_mut();
                for pic in pics.iter() {
                    pic.set_width_request(WIDE_EDGE_LENGTH);
                    pic.set_height_request(WIDE_EDGE_LENGTH);
                }
             },
         }
     }
}

impl YearsAlbum {
    fn refresh(&mut self) {
        let all_pictures = {
            let data = self.state.read();
            data
                .iter()
                .dedup_by(|x, y| x.year() == y.year())
                .map(|picture| PhotoGridItem {
                    picture: picture.clone(),
                    thumbnails: self.thumbnails.clone(),
                })
                .collect::<Vec<PhotoGridItem>>()
        };

        self.thumbnails.borrow_mut().clear();
        self.photo_grid.clear();
        self.photo_grid.extend_from_iter(all_pictures);

        if !self.photo_grid.is_empty() {
            self.photo_grid.view.scroll_to(
                self.photo_grid.len() - 1,
                gtk::ListScrollFlags::SELECT,
                None,
            );
        }
    }
}
