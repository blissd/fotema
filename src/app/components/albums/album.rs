// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
use fotema_core::YearMonth;
use fotema_core::visual::model::PictureOrientation;
use fotema_core::thumbnailify::{Thumbnailer, ThumbnailSize};

use gtk::prelude::OrientableExt;
use relm4::binding::*;
use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::gdk_pixbuf;
use relm4::gtk::prelude::AdjustmentExt;
use relm4::gtk::prelude::*;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use std::path::Path;
use std::sync::Arc;
use std::rc::Rc;
use strum::IntoEnumIterator;

use super::album_filter::AlbumFilter;
use super::album_sort::AlbumSort;
use crate::app::ActiveView;
use crate::app::SharedState;
use crate::app::ViewName;
use crate::app::adaptive;

use tracing::{debug, info};

const NARROW_EDGE_LENGTH: i32 = 112;
const WIDE_EDGE_LENGTH: i32 = 200;

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

    // Sort
    Sort(AlbumSort),

    // Adapt to layout
    Adapt(adaptive::Layout),

    // Scroll offset, in pixels.
    ScrollOffset(f64),

    // Scroll to top of photo grid, regardless of sort order
    ScrollToTop,
}

#[derive(Debug)]
pub enum AlbumOutput {
    /// User has selected photo or video in grid view
    Selected(VisualId, AlbumFilter),

    // Scroll offset, in pixels.
    ScrollOffset(f64),
}

#[derive(Debug)]
struct PhotoGridItem {
    visual: Arc<fotema_core::visual::Visual>,

    // Length of thumbnail edge to allow for resizing when layout changes.
    edge_length: I32Binding,

    thumbnailer: Rc<Thumbnailer>,
}

struct PhotoGridItemWidgets {
    picture: gtk::Picture,
    status_overlay: gtk::Frame,
    motion_type_icon: gtk::Image,
    duration_overlay: gtk::Frame,
    duration_label: gtk::Label,

    // If the gtk::Picture has been bound to edge_length.
    is_bound: bool,
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::Frame;
    type Widgets = PhotoGridItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            root = gtk::Frame {
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
                        set_content_fit: gtk::ContentFit::Cover,
                        set_width_request: NARROW_EDGE_LENGTH,
                        set_height_request: NARROW_EDGE_LENGTH,
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
            is_bound: false,
        };

        (root, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        // Bindings to allow dynamic update of thumbnail width and height
        // when layout changes between wide and narrow

        // If we repeatedly bind, then Fotema will die with the following error:
        // (fotema:2): GLib-GObject-CRITICAL **: 13:26:14.297: Too many GWeakRef registered
        // GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        // Bail out! GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)

        if !widgets.is_bound {
            widgets
                .picture
                .add_write_only_binding(&self.edge_length, "width-request");
            widgets
                .picture
                .add_write_only_binding(&self.edge_length, "height-request");
            widgets.is_bound = true;
        }

        let thumbnail_path = self.thumbnailer
            .nearest_thumbnail(&self.visual.thumbnail_hash(), ThumbnailSize::Large);

        if thumbnail_path.is_some() {
            widgets
                .picture
                .set_filename(thumbnail_path);
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/app/fotema/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                200,
                200,
                true,
            )
            .unwrap();
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

            let hhmmss = self
                .visual
                .video_duration
                .map(|ref x| fotema_core::time::format_hhmmss(x))
                .unwrap_or(String::from("—"));

            widgets.duration_label.set_label(&hhmmss);
        } else if self.visual.is_video_only() {
            widgets.status_overlay.set_visible(true);
            widgets.duration_overlay.set_visible(false);
            widgets
                .motion_type_icon
                .set_icon_name(Some("play-symbolic"));
        } else {
            // is_photo_only()
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
    sort: AlbumSort,
    edge_length: I32Binding,
    thumbnailer: Rc<Thumbnailer>,
}

#[relm4::component(pub)]
impl SimpleComponent for Album {
    type Init = (SharedState, ActiveView, ViewName, AlbumFilter, Rc<Thumbnailer>);
    type Input = AlbumInput;
    type Output = AlbumOutput;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,

            #[local_ref]
            grid_view -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,

                connect_activate[sender] => move |_, idx| {
                    sender.input(AlbumInput::Selected(idx))
                },
            },

            #[wrap(Some)]
            set_vadjustment = &gtk::Adjustment {
                // Emit scroll events so PersonAlbum can determine when to hide avatar.
                // FIXME maybe just emit one event at a boundary, instead of emitting an
                // event for every scroll?
                connect_value_changed[sender] => move |v| sender.input(AlbumInput::ScrollOffset(v.value())),
            },

        }
    }

    fn init(
        (state, active_view, view_name, filter, thumbnailer): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();
        let grid_view = &photo_grid.view.clone();

        let mut model = Album {
            state,
            active_view,
            view_name,
            photo_grid,
            filter,
            sort: AlbumSort::default(),
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
            thumbnailer,
        };

        model.update_filter();

        let widgets = view_output!();
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
                    info!("{:?} view is active so refreshing", self.view_name);
                    self.refresh();
                } else {
                    info!("{:?} view is inactive so clearing", self.view_name);
                    self.photo_grid.clear();
                }
            }
            AlbumInput::Filter(filter) => {
                self.filter = filter;
                self.update_filter();
                //self.scroll();
            }
            AlbumInput::Sort(sort) => {
                if self.sort != sort {
                    info!("Sort order is now {:?}", sort);
                    self.sort = sort;
                    sender.input(AlbumInput::Refresh);
                }
            }
            AlbumInput::Selected(index) => {
                // Albums are filters so must use get_visible(...) over get(...), otherwise
                // wrong photo is displayed.
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let visual_id = item.borrow().visual.visual_id.clone();
                    debug!("index {} has visual_id {}", index, visual_id);
                    let _ = sender.output(AlbumOutput::Selected(visual_id, self.filter.clone()));
                }
            }
            AlbumInput::GoToMonth(ym) => {
                info!("Showing for month: {}", ym);
                let index_opt = self.photo_grid.find(|p| p.visual.year_month() == ym);
                if let Some(index) = index_opt {
                    let flags = gtk::ListScrollFlags::SELECT;
                    debug!("Scrolling to {}", index);
                    self.photo_grid.view.scroll_to(index, flags, None);
                }
            }
            AlbumInput::ScrollToTop => {
                // Hmm... not sure I like this...
                if !self.photo_grid.is_empty() {
                    self.photo_grid
                        .view
                        .scroll_to(0, gtk::ListScrollFlags::SELECT, None);
                }
            }
            AlbumInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            }
            AlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            }
            AlbumInput::ScrollOffset(offset) => {
                let _ = sender.output(AlbumOutput::ScrollOffset(offset));
            }
        }
    }
}

impl Album {
    fn refresh(&mut self) {
        let mut all = {
            let data = self.state.read();
            data.iter()
                .map(|visual| PhotoGridItem {
                    visual: visual.clone(),
                    edge_length: self.edge_length.clone(),
                    thumbnailer: self.thumbnailer.clone(),
                })
                .collect::<Vec<PhotoGridItem>>()
        };

        // State is always in ascending time order
        self.sort.sort(&mut all);

        self.photo_grid.clear();

        //self.photo_grid.add_filter(move |item| (self.photo_grid_filter)(&item.picture));
        self.photo_grid.extend_from_iter(all);

        info!("{} items added to album", self.photo_grid.len());

        // NOTE person album will in effect overide scrolling to the end
        // by sending a ScrollToTop command.
        self.sort.scroll_to_end(&mut self.photo_grid);
    }

    fn update_filter(&mut self) {
        self.photo_grid.clear_filters();
        let filter = self.filter.clone();
        self.photo_grid
            .add_filter(move |item| filter.clone().filter(&item.visual));
    }
}
