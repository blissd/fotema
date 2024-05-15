// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use fotema_core::VisualId;
use fotema_core::YearMonth;
use fotema_core::visual::model::PictureOrientation;
use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::prelude::*;
use relm4::gtk::gdk_pixbuf;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use std::path::Path;
use std::sync::Arc;

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
}

#[derive(Debug)]
pub enum AlbumOutput {
    /// User has selected photo or video in grid view
    Selected(VisualId, AlbumFilter),
}

#[derive(Debug)]
struct PhotoGridItem {
    visual: Arc<fotema_core::visual::Visual>,
}

struct PhotoGridItemWidgets {
    picture: gtk::Picture,
    status_overlay: gtk::Frame,
    motion_type_icon: gtk::Image,
    duration_overlay: gtk::Frame,
    duration_label: gtk::Label,
}

impl RelmGridItem for PhotoGridItem {
    type Root = adw::Clamp;
    type Widgets = PhotoGridItemWidgets;

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            root = adw::Clamp {
                set_maximum_size: 200,
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
        if self.visual.thumbnail_path.as_ref().is_some_and(|x| x.exists()) {
            widgets.picture.set_filename(self.visual.thumbnail_path.clone());
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

            let total_seconds = self.visual.video_duration.expect("must have video duration").num_seconds();
            let seconds = total_seconds % 60;
            let minutes = (total_seconds / 60) % 60;
            let hours = (total_seconds / 60) / 60;
            let hhmmss = if hours == 0 {
                format!("{}:{:0>2}", minutes, seconds)
            } else {
                format!("{}:{:0>2}:{:0>2}", hours, minutes, seconds)
            };
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

            // Add CSS class for orientation
            let orientation = self.visual.picture_orientation.unwrap_or(PictureOrientation::North);
            widgets.picture.add_css_class(orientation.as_ref());

        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&Path>);
        widgets.motion_type_icon.set_icon_name(None);
        widgets.status_overlay.set_visible(false);
        widgets.duration_overlay.set_visible(false);
        widgets.duration_label.set_label("");

        let orientation = self.visual.picture_orientation.unwrap_or(PictureOrientation::North);
        widgets.picture.add_css_class(orientation.as_ref());
    }
}

pub struct Album {
    state: SharedState,
    active_view: ActiveView,
    view_name: ViewName,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
    filter: AlbumFilter,
}

#[relm4::component(pub)]
impl SimpleComponent for Album {
    type Init = (SharedState, ActiveView, ViewName, AlbumFilter);
    type Input = AlbumInput;
    type Output = AlbumOutput;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,

            #[local_ref]
            grid_view -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,
                //set_max_columns: 3,

                connect_activate[sender] => move |_, idx| {
                    sender.input(AlbumInput::Selected(idx))
                },
            }
        }
    }

    fn init(
        (state, active_view, view_name, filter): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let mut model = Album { state, active_view, view_name, photo_grid, filter };

        model.update_filter();

        let grid_view = &model.photo_grid.view;

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
            }
        }
    }
}

impl Album {

    fn refresh(&mut self) {
        let all = {
            let data = self.state.read();
            data
                .iter()
                .map(|visual| PhotoGridItem { visual: visual.clone() })
                .collect::<Vec<PhotoGridItem>>()
        };

        self.photo_grid.clear();

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
