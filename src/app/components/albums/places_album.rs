// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core;

use itertools::Itertools;

use relm4::gtk;
use relm4::gtk::prelude::FrameExt;
use relm4::gtk::prelude::WidgetExt;
use relm4::gtk::gdk_pixbuf;
use relm4::gtk::gdk;
use relm4::*;
use relm4::binding::*;

use tracing::{debug,error,info};

use crate::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;
use fotema_core::{Visual, VisualId};

use h3o;
use h3o::CellIndex;

use shumate;
use shumate::prelude::*;
use shumate::MAP_SOURCE_OSM_MAPNIK;

const NARROW_EDGE_LENGTH: i32 = 60;
const WIDE_EDGE_LENGTH: i32 = 100;

const MIN_ZOOM_LEVEL: u32 = 3;
const MAX_ZOOM_LEVEL: u32 = 17;
const DEFAULT_ZOOM_LEVEL: f64 = 5.0;

#[derive(Debug)]
pub enum PlacesAlbumInput {
    Activate,

    // Reload photos from database
    Refresh,

    // Adapt to layout
    Adapt(adaptive::Layout),

    // Map zoom has changed
    Zoom,
}

#[derive(Debug)]
pub enum PlacesAlbumOutput {
    /// User has selected a single item to view on map
    View(VisualId),

    // User has selected a group of items grouped in a cell index to view as an album
    GeographicArea(CellIndex),
}

pub struct PlacesAlbum {
    state: SharedState,
    active_view: ActiveView,
    edge_length: I32Binding,

    /// Map of visual items
    map: shumate::SimpleMap,
    viewport: shumate::Viewport,

    /// Layer containing thumbnails
    marker_layer: shumate::MarkerLayer,

    /// Current resolution being viewed
    resolution: h3o::Resolution,

    need_refresh: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for PlacesAlbum {
    type Init = (SharedState, ActiveView);
    type Input = PlacesAlbumInput;
    type Output = PlacesAlbumOutput;

    view! {
       gtk::Box {
       #[local_ref]
        map_widget -> shumate::SimpleMap{
            set_vexpand: true,
            set_hexpand: true,
        },
       },
    }

    fn init(
        (state, active_view): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        //let map = Map::new();
        let map_widget = shumate::SimpleMap::builder()
           // .connect_scale_notify(|_| println!("scale"))
            .build();

        if let Some(scale) = map_widget.scale() {
            scale.set_unit(shumate::Unit::Metric);
        }

        // Use OpenStreetMap as the source
        let registry = shumate::MapSourceRegistry::with_defaults();
        let map_source = registry.by_id(MAP_SOURCE_OSM_MAPNIK);
        let map = map_widget.map().unwrap();

        map_widget.set_map_source(map_source.as_ref());

        // Reference map source used by MarkerLayer
        let viewport = map_widget.viewport().unwrap();
        viewport.set_reference_map_source(map_source.as_ref());
        viewport.set_min_zoom_level(MIN_ZOOM_LEVEL);
        viewport.set_max_zoom_level(MAX_ZOOM_LEVEL);
        viewport.set_zoom_level(DEFAULT_ZOOM_LEVEL);
        viewport.connect_zoom_level_notify(move |_| sender.input(PlacesAlbumInput::Zoom));
        //viewport.connect_latitude_notify(|_| sender.input(PlacesAlbumInput::Zoom);
        //viewport.connect_longitude_notify(|_| sender.input(PlacesAlbumInput::Zoom);

        let gesture = gtk::GestureClick::new();
        map_widget.add_controller(gesture.clone());

        let marker_layer: shumate::MarkerLayer =
            shumate::MarkerLayer::new_full(&viewport, gtk::SelectionMode::Single);

        //let marker = shumate::Marker::new();
        //marker.set_location(0., 0.);
        //marker_layer.add_marker(&marker);

        map.add_layer(&marker_layer);

        let model = PlacesAlbum {
            state,
            active_view,
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
            map: map_widget.clone(),
            viewport,
            marker_layer,
            resolution: h3o::Resolution::Five,
            need_refresh: true,
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PlacesAlbumInput::Activate => {
                *self.active_view.write() = ViewName::Places;
                if self.need_refresh {
                    self.refresh(&sender);
                }
            }
            PlacesAlbumInput::Refresh => {
                if *self.active_view.read() == ViewName::Places {
                    info!("Places view is active so refreshing");
                    self.refresh(&sender);
                } else {
                    info!("Places view is inactive so clearing");
                    self.marker_layer.remove_all();
                    self.need_refresh = true;
                }
            }
            PlacesAlbumInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            },
            PlacesAlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            },
            PlacesAlbumInput::Zoom => {
                let zoom_level = self.map.viewport().unwrap().zoom_level();
                debug!("zoom level = {}", zoom_level);
                self.update_layer(&PlacesAlbum::zoom_to_resolution(zoom_level), &sender);
            },
        }
    }
}

impl PlacesAlbum {

    /// Maps a Shumate zoom level to a H3O resolution
    /// FIXME this is pretty coarse. Would be good map by scale or by fractional zoom levels.
    fn zoom_to_resolution(zoom_level: f64) -> h3o::Resolution {
        match zoom_level as u32 {
            a if a <= MIN_ZOOM_LEVEL => h3o::Resolution::Zero,
            4 => h3o::Resolution::One,
            5 => h3o::Resolution::Two,
            6 => h3o::Resolution::Two,
            7 => h3o::Resolution::Three,
            8 => h3o::Resolution::Four,
            9 => h3o::Resolution::Four,
            10 => h3o::Resolution::Five,
            11 => h3o::Resolution::Six,
            12 => h3o::Resolution::Six,
            13 => h3o::Resolution::Seven,
            14 => h3o::Resolution::Eight,
            15 => h3o::Resolution::Eight,
            16 => h3o::Resolution::Nine,
            a if a >= MAX_ZOOM_LEVEL => h3o::Resolution::Nine,
            _ =>  h3o::Resolution::Five,
        }
    }

    fn update_layer(&mut self, resolution: &h3o::Resolution, sender: &ComponentSender<Self>) {
        if self.resolution == *resolution {
            return;
        }

        self.resolution = *resolution;

        self.marker_layer.remove_all();

        let data = self.state.read();
        data.iter()
            // only want visual items with location
            .filter(|x| x.location.is_some())
            // make visual items in same cell adjacent
            .sorted_by_key(|x| x.location.map(|y| y.to_cell(*resolution)))
            // group visual items in same cell
            .chunk_by(|x| x.location.map(|y| y.to_cell(*resolution)))
            .into_iter()
            .for_each(|(_cell_index, vs)| {
                let mut vs = vs.collect_vec();

                // Use newest visual item for thumbnail
                vs.sort_by_key(|x| x.ordering_ts);
                let item = vs.last().expect("Groups can't be empty");

                let widget = self.to_pin_thumbnail(&item, Some(vs.len()), sender);

                let marker = shumate::Marker::builder()
                    .child(&widget)
                    .build();

                // Hmmm... using the cell_index lat/lng can put thumbnails kinda far from where
                // they occured
                //let location: LatLng = cell_index.into();
                let Some(location) = item.location else {
                    error!("Empty location after grouping");
                    return;
                };

                marker.set_location(location.lat(), location.lng());

               self.marker_layer.add_marker(&marker);
            });
    }

    fn refresh(&mut self, sender: &ComponentSender<Self>) {
        let data = self.state.read().clone();
        let data = data.iter().filter(|x| x.location.is_some()).collect_vec();

        info!("{} items with location data", data.len());

        if let Some(most_recent) = data.iter().max_by(|x,y|x.ordering_ts.cmp(&y.ordering_ts)) {
            let location = most_recent.location.expect("must have location");
            info!("Centreing on most recent location at {}", location);
            let map = self.map.map().expect("must have map");
            map.center_on(location.lat(), location.lng());
        }

        self.viewport.set_zoom_level(DEFAULT_ZOOM_LEVEL);
        self.update_layer(&PlacesAlbum::zoom_to_resolution(DEFAULT_ZOOM_LEVEL), sender);
        self.need_refresh = false;
    }

    /// Make thumbnail to put onto map
    fn to_pin_thumbnail(&self, visual: &Visual, count: Option<usize>, sender: &ComponentSender<PlacesAlbum>) -> gtk::Frame {
        let picture = if visual.thumbnail_path.as_ref().is_some_and(|x| x.exists()) {
            let picture = gtk::Image::from_file(visual.thumbnail_path.as_ref().expect("Must have path"));

            // Add CSS class for orientation
            let orientation = visual.thumbnail_orientation();
            picture.add_css_class(orientation.as_ref());
            picture
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/app/fotema/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                200, 200, true
            ).unwrap();
           let img = gdk::Texture::for_pixbuf(&pb);
           let picture = gtk::Image::from_paintable(Some(&img));
            picture
        };

        picture.add_write_only_binding(&self.edge_length, "width-request");
        picture.add_write_only_binding(&self.edge_length, "height-request");

        //picture.set_width_request(100);
        //picture.set_height_request(100);

        let frame = gtk::Frame::new(None);

        let count = count.unwrap_or(1);

         if count > 1 {
            // if there is a count then overlay the number in the bottom right corner.
            let overlay = gtk::Overlay::builder()
                .child(&picture)
                .build();

            let label = gtk::Label::builder()
                .label(format!("{}", count))
                .css_classes(["photo-grid-photo-status-label"]) // FIXME don't reuse CSS class.
                .build();

            let label_frame = gtk::Frame::builder()
                .halign(gtk::Align::End)
                .valign(gtk::Align::End)
                .css_classes(["photo-grid-photo-status-frame"]) // FIXME don't reuse CSS class.
                .child(&label)
                .build();

            label_frame.set_margin_all(4);

            overlay.add_overlay(&label_frame);

            frame.set_child(Some(&overlay));
           // frame
        } else {
            frame.set_child(Some(&picture));
        }

        frame.add_css_class("map-thumbnail-border");

        let click = gtk::GestureClick::new();
        {
            let visual = visual.clone();
            let sender = sender.clone();
            let resolution = self.resolution;
            click.connect_released(move |_click,_,_,_| {
                if count > 1 {
                    info!("Viewing album containing: {}", visual.visual_id);
                    if let Some(cell_index) = visual.location.map(|loc| loc.to_cell(resolution)) {
                        let _ = sender.output(PlacesAlbumOutput::GeographicArea(cell_index));
                    }
                } else {
                    info!("Viewing item: {}", visual.visual_id);
                    let _ = sender.output(PlacesAlbumOutput::View(visual.visual_id.clone()));
                }
            });
        }

        {
            // if we get a stop message, then we are not dealing with a single-click.
            click.connect_stopped(move |click| click.reset());
        }

        frame.add_controller(click);

        frame
    }
}

