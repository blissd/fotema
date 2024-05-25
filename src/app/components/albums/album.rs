// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use fotema_core::VisualId;
use fotema_core::YearMonth;
use fotema_core::visual::model::PictureOrientation;
use strum::IntoEnumIterator;
use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::prelude::*;
use relm4::gtk::gdk_pixbuf;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use std::path::Path;
use std::sync::Arc;
use std::sync::RwLock;
use std::rc::Rc;
use std::collections::HashSet;

use crate::app::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;
use super::album_filter::AlbumFilter;

use tracing::{event, Level};


#[derive(Debug)]
pub enum AlbumInput {

    /// Album is visible
    Activate,

    // State has been updated
    Refresh,

    /// User has selected photo in grid view
    Selected(u32), // Index into a Vec

    // Scroll to first photo of year/month.
    GoToMonth(YearMonth),

    // I'd like to pass a closure of Fn(Picture)->bool for the filter... but Rust
    // is making that too hard.

    // Show no photos
    Filter(AlbumFilter),

    // Adapt to layout
    Adapt(adaptive::Layout),
}

#[derive(Debug)]
pub enum AlbumOutput {
    /// User has selected photo or video in grid view
    Selected(VisualId, AlbumFilter),
}

#[derive(Debug)]
struct PhotoGridItem {
    visual: Arc<fotema_core::visual::Visual>,

    // Supports dynamic resizing of thumbnails
    all_pictures: Rc<RwLock<HashSet<gtk::Picture>>>
}

struct PhotoGridItemWidgets {
    picture: gtk::Picture,
    status_overlay: gtk::Frame,
    motion_type_icon: gtk::Image,
    duration_overlay: gtk::Frame,
    duration_label: gtk::Label,
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::AspectFrame;
    type Widgets = PhotoGridItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            root = gtk::AspectFrame {
                gtk::Frame {
                    gtk::Overlay {
                        #[name(status_overlay)]
                        add_overlay =  &gtk::Frame {
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::End,
                            set_margin_all: 8,
                            add_css_class: "photo-grid-photo-status-frame",

                            #[wrap(Some)]
                            #[name(motion_type_icon)]
                            set_child = &gtk::Image {
                                set_width_request: 16,
                                set_height_request: 16,
                                add_css_class: "photo-grid-photo-status-label",
                            },
                        },

                        #[name(duration_overlay)]
                        add_overlay =  &gtk::Frame {
                            set_halign: gtk::Align::End,
                            set_valign: gtk::Align::End,
                            set_margin_all: 8,
                            add_css_class: "photo-grid-photo-status-frame",

                            #[wrap(Some)]
                            #[name(duration_label)]
                            set_child = &gtk::Label{
                                add_css_class: "photo-grid-photo-status-label",
                            },
                        },

                        #[wrap(Some)]
                        #[name(picture)]
                        set_child = &gtk::Picture {
                            set_can_shrink: true,
                        }
                    }
                }
            }
        }

        let widgets = PhotoGridItemWidgets {
            picture,
            status_overlay,
            motion_type_icon,
            duration_overlay,
            duration_label,
        };

        (root, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {

        // Add our picture to the set of all pictures so it can be easily resized
        // when the window dimensions changes between wide and narrow.
        if !self.all_pictures.read().expect("Lock").contains(&widgets.picture) {
            self.all_pictures.write().expect("Lock").insert(widgets.picture.clone());
        }

        if self.visual.thumbnail_path.as_ref().is_some_and(|x| x.exists()) {
            widgets.picture.set_filename(self.visual.thumbnail_path.clone());

            // Add CSS class for orientation
            let orientation = self.visual.thumbnail_orientation();
            widgets.picture.add_css_class(orientation.as_ref());
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/app/fotema/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                200, 200, true
            ).unwrap();
           let img = gdk::Texture::for_pixbuf(&pb);
            widgets.picture.set_paintable(Some(&img));
        }

        if self.visual.is_motion_photo() {
            widgets.status_overlay.set_visible(true);
            widgets.duration_overlay.set_visible(false);
            widgets.duration_label.set_label("");
            widgets.motion_type_icon.set_icon_name(Some("cd-symbolic"));
        } else if self.visual.is_video_only() && self.visual.video_duration.is_some() {
            widgets.status_overlay.set_visible(false);
            widgets.duration_overlay.set_visible(true);

            let hhmmss = self.visual
                .video_duration
                .map(|ref x| fotema_core::time::format_hhmmss(x))
                .unwrap_or(String::from("—"));

            widgets.duration_label.set_label(&hhmmss);
        } else if self.visual.is_video_only() {
            widgets.status_overlay.set_visible(true);
            widgets.duration_overlay.set_visible(false);
            widgets.motion_type_icon.set_icon_name(Some("play-symbolic"));
        } else { // is_photo_only()
            widgets.status_overlay.set_visible(false);
            widgets.motion_type_icon.set_icon_name(None);
            widgets.duration_overlay.set_visible(false);
            widgets.duration_label.set_label("");
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&Path>);
        widgets.motion_type_icon.set_icon_name(None);
        widgets.status_overlay.set_visible(false);
        widgets.duration_overlay.set_visible(false);
        widgets.duration_label.set_label("");

        // clear orientation transformation css classes
        for orient in PictureOrientation::iter() {
            widgets.picture.remove_css_class(orient.as_ref());
        }
    }
}

pub struct Album {
    state: SharedState,
    active_view: ActiveView,
    view_name: ViewName,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
    filter: AlbumFilter,
    layout: adaptive::Layout,
    item_pictures: Rc<RwLock<HashSet<gtk::Picture>>>,
}

pub struct AlbumWidgets {
    grid_view: gtk::GridView,

    // All pictures referenced by grid view.
    item_pictures: Rc<RwLock<HashSet<gtk::Picture>>>,
}

//#[relm4::component(pub)]
impl SimpleComponent for Album {
    type Init = (SharedState, ActiveView, ViewName, AlbumFilter);
    type Input = AlbumInput;
    type Output = AlbumOutput;
    type Root = gtk::ScrolledWindow;
    type Widgets = AlbumWidgets;

    fn init_root() -> Self::Root {

        gtk::ScrolledWindow::builder()
            .vexpand(true)
            .build()
    }

    fn init(
        (state, active_view, view_name, filter): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

         let grid_view = &photo_grid.view;
         grid_view.set_orientation(gtk::Orientation::Vertical);
         grid_view.set_single_click_activate(true);
         grid_view.connect_activate(move |_, idx| sender.input(AlbumInput::Selected(idx)));

        let mut model = Album {
            state,
            active_view,
            view_name,
            photo_grid,
            filter,
            layout: adaptive::Layout::Narrow,
            item_pictures: Rc::new(RwLock::new(HashSet::new())),
        };

        model.update_filter();

        let grid_view = &model.photo_grid.view;

        let widgets = AlbumWidgets {
            grid_view: grid_view.clone(),
            item_pictures: model.item_pictures.clone(),
        };

        root.set_child(Some(&model.photo_grid.view));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AlbumInput::Activate => {
                *self.active_view.write() = self.view_name;
                if self.photo_grid.is_empty() {
                    self.refresh();
                }
            }
            AlbumInput::Refresh => {
                if *self.active_view.read() == self.view_name {
                    self.refresh();
                } else {
                    self.photo_grid.clear();
                }
            }
            AlbumInput::Filter(filter) => {
                self.filter = filter;
                self.update_filter();
            }
            AlbumInput::Selected(index) => {
                // Photos are filters so must use get_visible(...) over get(...), otherwise
                // wrong photo is displayed.
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let visual_id = item.borrow().visual.visual_id.clone();
                    event!(Level::DEBUG, "index {} has visual_id {}", index, visual_id);
                    let _ = sender.output(AlbumOutput::Selected(visual_id, self.filter.clone()));
                }
            }
            AlbumInput::GoToMonth(ym) => {
                event!(Level::INFO, "Showing for month: {}", ym);
                let index_opt = self.photo_grid.find(|p| p.visual.year_month() == ym);
                event!(Level::DEBUG, "Found: {:?}", index_opt);
                if let Some(index) = index_opt {
                    let flags = gtk::ListScrollFlags::SELECT;
                    event!(Level::DEBUG, "Scrolling to {}", index);
                    self.photo_grid.view.scroll_to(index, flags, None);
                }
            },
            AlbumInput::Adapt(layout @ adaptive::Layout::Narrow) => {
                event!(Level::DEBUG, "Adapt narrow");
                self.layout = layout;
            },
            AlbumInput::Adapt(layout @ adaptive::Layout::Wide) => {
                event!(Level::DEBUG, "Adapt wide");
                self.layout = layout;
            },
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        match self.layout {
            // Update thumbnail size depending on adaptive layout type
            adaptive::Layout::Narrow => {
                let pics = widgets.item_pictures.write().expect("Lock to adapt narrow");
                for pic in pics.iter() {
                    pic.set_width_request(110);
                    pic.set_height_request(110);
                }
            },
            adaptive::Layout::Wide => {
                let pics = widgets.item_pictures.write().expect("Lock to adapt wide");
                for pic in pics.iter() {
                    pic.set_width_request(170);
                    pic.set_height_request(170);
                }
            },
        }
    }
}

impl Album {

    fn refresh(&mut self) {
        let all = {
            let data = self.state.read();
            data
                .iter()
                .map(|visual| PhotoGridItem {
                    visual: visual.clone(),
                    all_pictures: self.item_pictures.clone(),
                })
                .collect::<Vec<PhotoGridItem>>()
        };

        self.photo_grid.clear();
        self.item_pictures.write().expect("Lock for clear").clear();

        //self.photo_grid.add_filter(move |item| (self.photo_grid_filter)(&item.picture));
        self.photo_grid.extend_from_iter(all);

        if !self.photo_grid.is_empty() {
            // We must scroll to a valid index... but we can't get the index of the
            // last item if filters are enabled. So as a workaround disable filters,
            // scroll to end, and then enable filters.

            self.disable_filters();

            self.photo_grid.view.scroll_to(
                self.photo_grid.len() - 1,
                gtk::ListScrollFlags::SELECT,
                None,
            );

            self.enable_filters();
        }
    }

    fn disable_filters(&mut self) {
        for i in 0..(self.photo_grid.filters_len()) {
            self.photo_grid.set_filter_status(i, false);
        }
    }

    fn enable_filters(&mut self) {
        for i in 0..(self.photo_grid.filters_len()) {
            self.photo_grid.set_filter_status(i, true);
        }
    }

    fn update_filter(&mut self) {
        self.photo_grid.clear_filters();
        let filter = self.filter.clone();
        self.photo_grid.add_filter(move |item| filter.clone().filter(&item.visual));
    }
}
