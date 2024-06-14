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
use relm4::binding::*;

use std::path;
use std::sync::Arc;

use tracing::info;

use crate::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;

use shumate::{Map};
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
}

pub struct PlacesAlbum {
    state: SharedState,
    active_view: ActiveView,
    edge_length: I32Binding,
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
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        //let map = Map::new();
        let map_widget = shumate::SimpleMap::new();

        // Use OpenStreetMap as the source
        let registry = shumate::MapSourceRegistry::with_defaults();
        let map_source = registry.by_id(MAP_SOURCE_OSM_MAPNIK);
        let map = map_widget.map().unwrap();

        map_widget.set_map_source(map_source.as_ref());
        map.center_on(0., 0.);

        // Reference map source used by MarkerLayer
        let viewport = map_widget.viewport().unwrap();
        viewport.set_reference_map_source(map_source.as_ref());
        viewport.set_zoom_level(5.);

        let gesture = gtk::GestureClick::new();
        map_widget.add_controller(gesture.clone());

        let marker_layer: shumate::MarkerLayer =
            shumate::MarkerLayer::new_full(&viewport, gtk::SelectionMode::Single);

        let marker = shumate::Marker::new();
        marker.set_location(0., 0.);
        marker_layer.add_marker(&marker);
        map.add_layer(&marker_layer);

        let model = PlacesAlbum {
            state,
            active_view,
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
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
        }
    }
}

impl PlacesAlbum {
    fn refresh(&mut self) {
        let all_pictures = {
            let data = self.state.read();

        };

    }
}

