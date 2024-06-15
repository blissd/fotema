// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core;

use strum::IntoEnumIterator;

use itertools::Itertools;

use relm4::gtk;
use relm4::gtk::prelude::FrameExt;
use relm4::gtk::prelude::WidgetExt;
use relm4::gtk::gdk_pixbuf;
use relm4::gtk::gdk;
use relm4::*;
use relm4::binding::*;

use std::path;
use std::sync::Arc;

use tracing::info;

use crate::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;
use fotema_core::Visual;

use shumate::Map;
use shumate;
use shumate::prelude::*;
use shumate::MAP_SOURCE_OSM_MAPNIK;
use shumate::{MAX_LATITUDE, MAX_LONGITUDE, MIN_LATITUDE, MIN_LONGITUDE};


const NARROW_EDGE_LENGTH: i32 = 170;
const WIDE_EDGE_LENGTH: i32 = 200;

#[derive(Debug)]
pub enum PlacesAlbumInput {
    Activate,

    // Reload photos from database
    Refresh,

    // Adapt to layout
    Adapt(adaptive::Layout),

    Map,
    MapSource,
    Scale,
    Viewport,
    Zoom,
}

pub struct PlacesAlbum {
    state: SharedState,
    active_view: ActiveView,
    edge_length: I32Binding,
    map: shumate::SimpleMap,
    viewport: shumate::Viewport,
    marker_layer: shumate::MarkerLayer,
}

#[relm4::component(pub)]
impl SimpleComponent for PlacesAlbum {
    type Init = (SharedState, ActiveView);
    type Input = PlacesAlbumInput;
    type Output = ();

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
        viewport.set_zoom_level(5.);
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
        };

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PlacesAlbumInput::Activate => {
                *self.active_view.write() = ViewName::Places;
                self.refresh();
            }
            PlacesAlbumInput::Refresh => {
                if *self.active_view.read() == ViewName::Places {
                    info!("Places view is active so refreshing");
                    self.refresh();
                } else {
                    info!("Places view is inactive so clearing");
                    //self.photo_grid.clear();
                }
            }
            PlacesAlbumInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            },
            PlacesAlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            },

            PlacesAlbumInput::Zoom => {
                println!("zoom level = {}", self.map.viewport().unwrap().zoom_level());
            },
            PlacesAlbumInput::Map => {
                println!("Map!");
            },
            PlacesAlbumInput::MapSource => {
                println!("MapSource!");
            },
            PlacesAlbumInput::Scale => {
                println!("Scale!");
            },
            PlacesAlbumInput::Viewport => {
                println!("Viewport!");
            },
        }
    }
}

impl PlacesAlbum {
    fn refresh(&mut self) {
        let data = self.state.read();
        let data = data.iter().filter(|x| x.location.is_some()).collect_vec();

        info!("{} items with location data", data.len());

        if let Some(most_recent) = data.iter().max_by(|x,y|x.ordering_ts.cmp(&y.ordering_ts)) {
            let location = most_recent.location.expect("must have location");
            info!("Centreing on most recent location at {}", location);
            self.map.map().expect("must have map").center_on(location.lat(), location.lng());

            let widget = to_pin_thumbnail(&most_recent);

            let marker = shumate::Marker::builder()
                .child(&widget)
                .build();

            marker.set_location(location.lat(), location.lng());

           self.marker_layer.add_marker(&marker)
        }
    }
}

/// Make thumbnail to put onto map
fn to_pin_thumbnail(visual: &Visual) -> gtk::Frame {
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

    picture.set_width_request(100);
    picture.set_height_request(100);

    let frame = gtk::Frame::builder()
        .child(&picture)
        .build();

    frame.add_css_class("map-thumbnail-border");

    frame
}
