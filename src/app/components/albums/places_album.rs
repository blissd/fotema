// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core;

use itertools::Itertools;

use relm4::binding::*;
use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::gdk_pixbuf;
use relm4::gtk::prelude::FrameExt;
use relm4::gtk::prelude::WidgetExt;
use relm4::*;

use tracing::{debug, error, info};

use crate::adaptive;
use crate::app::ActiveView;
use crate::app::SharedState;
use crate::app::ViewName;

use fotema_core::{Visual, VisualId};
use fotema_core::thumbnailify::{Thumbnailer, ThumbnailSize};

use h3o;
use h3o::CellIndex;

use shumate;
use shumate::MAP_SOURCE_OSM_MAPNIK;
use shumate::prelude::*;

use std::collections::HashMap;
use std::sync::Arc;
use std::rc::Rc;

const NARROW_EDGE_LENGTH: i32 = 60;
const WIDE_EDGE_LENGTH: i32 = 100;

const MIN_ZOOM_LEVEL: u32 = 3;
const MAX_ZOOM_LEVEL: u32 = 17;

const DEFAULT_ZOOM_LEVEL: f64 = 7.0;

#[derive(Debug)]
pub enum PlacesAlbumInput {
    Activate,

    // Reload photos from database
    Refresh,

    // Adapt to layout
    Adapt(adaptive::Layout),

    // Map zoom has changed
    Zoom,

    // Map has been dragged
    Move,
}

#[derive(Debug)]
pub enum PlacesAlbumOutput {
    /// User has selected a single item to view on map
    View(VisualId),

    // User has selected a group of items grouped in a cell index to view as an album
    GeographicArea(CellIndex),
}

/// Item to represent all photos in a cell
#[derive(Debug, Clone)]
pub struct CellItem {
    /// Visual item to use for thumbnail.C
    visual: Arc<Visual>,

    /// Count of visual items in cell. Used for label.
    count: usize,
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

    /// Cells at current resolution.
    /// Cell index, visual item for thumbnail, count of items in cell
    cells: HashMap<CellIndex, CellItem>,

    /// Cell nearest centre of map
    centre_cell: h3o::CellIndex,

    need_refresh: bool,
    thumbnailer: Rc<Thumbnailer>,
}

#[relm4::component(pub)]
impl SimpleComponent for PlacesAlbum {
    type Init = (SharedState, ActiveView, Rc<Thumbnailer>);
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
        (state, active_view, thumbnailer): Self::Init,
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

        {
            let sender = sender.clone();
            viewport.connect_zoom_level_notify(move |_| sender.input(PlacesAlbumInput::Zoom));
        }
        {
            let sender = sender.clone();
            viewport.connect_latitude_notify(move |_| sender.input(PlacesAlbumInput::Move));
        }
        {
            let sender = sender.clone();
            viewport.connect_longitude_notify(move |_| sender.input(PlacesAlbumInput::Move));
        }

        let gesture = gtk::GestureClick::new();
        map_widget.add_controller(gesture.clone());

        let marker_layer: shumate::MarkerLayer =
            shumate::MarkerLayer::new_full(&viewport, gtk::SelectionMode::Single);

        map.add_layer(&marker_layer);

        let model = PlacesAlbum {
            state,
            active_view,
            need_refresh: true,
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
            map: map_widget.clone(),
            viewport,
            marker_layer,

            // NOTE will immediately be overridden when map is first rendered
            resolution: h3o::Resolution::Zero,

            cells: HashMap::new(),

            // NOTE will immediately be overridden when map is first rendered
            centre_cell: h3o::LatLng::new(0.0, 0.0)
                .expect("0/0 is a valid lat/lng")
                .to_cell(h3o::Resolution::Zero),

            thumbnailer,
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
            }
            PlacesAlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            }
            PlacesAlbumInput::Zoom => {
                let zoom_level = self.viewport.zoom_level();
                debug!("zoom level = {}", zoom_level);
                self.update_on_zoom(&PlacesAlbum::zoom_to_resolution(zoom_level));
                // a zoom is also a move
                self.update_on_move(&sender);
            }
            PlacesAlbumInput::Move => {
                self.update_on_move(&sender);
            }
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
            _ => h3o::Resolution::Three,
        }
    }

    fn update_on_zoom(&mut self, resolution: &h3o::Resolution) {
        if self.resolution == *resolution {
            return;
        }

        // When the resolution changes the set of cells on the map changes
        // and the thumbnails and counts change.

        debug!(
            "Resolution zoomed from {} to {}",
            self.resolution, resolution
        );

        self.resolution = *resolution;

        let data = self.state.read();
        self.cells.clear();

        // Build a map of cell indexes to cell items for current resolution
        data.iter()
            // only want visual items with location
            .filter(|x| x.location.is_some())
            // make visual items in same cell adjacent
            .sorted_by_key(|x| x.location.map(|y| y.to_cell(*resolution)))
            // group visual items in same cell
            .chunk_by(|x| x.location.map(|y| y.to_cell(*resolution)))
            .into_iter()
            .filter(|(index, _)| index.is_some())
            .map(|(index, vs)| (index.unwrap(), vs))
            .for_each(|(cell_index, vs)| {
                let vs = vs.collect_vec();
                let count = vs.len();

                // Use newest visual item for thumbnail
                if let Some(visual) = vs.into_iter().max_by_key(|x| x.ordering_ts) {
                    let item = CellItem {
                        visual: visual.clone(),
                        count,
                    };
                    self.cells.insert(cell_index, item);
                }
            });
    }

    fn update_on_move(&mut self, sender: &ComponentSender<Self>) {
        // Note that zoom computes the cells on the map so should come before move
        // if a zoom and move has occurred.

        let Ok(centre_point) =
            h3o::LatLng::new(self.viewport.latitude(), self.viewport.longitude())
        else {
            error!(
                "Invalid viewport centre point: lat={}, lng={}",
                self.viewport.latitude(),
                self.viewport.longitude()
            );
            return;
        };

        let centre_cell = centre_point.to_cell(self.resolution);

        if self.centre_cell == centre_cell {
            return;
        }

        // When the centre cell changes then the set of visible cells changes.
        // Some new cells will become visible, and other cells will no longer be visible.
        debug!(
            "Centre cell moved from {} to {}",
            self.centre_cell, centre_cell
        );

        self.centre_cell = centre_cell;

        // Get neighbouring cells. Hopefully enough to fully cover the map,
        // but not so many that the UI stutters.
        let nearby = centre_cell.grid_disk::<Vec<_>>(5);
        debug!("{} cells near centre", nearby.len());

        // WARNING reusing the marker layer by removing all markers and then adding new ones
        // would result in crashes (without any stack traces or logging) when viewing some images.
        // Not 100% sure of the cause or if the libshumate underlying C code is responsible, but
        // that is my best guess for now.
        //
        // As a work around I'll remove the old marker layer and add a new one.
        self.marker_layer.remove_all();

        let map = self.map.map().expect("Must have map");
        map.remove_layer(&self.marker_layer);

        self.marker_layer =
            shumate::MarkerLayer::new_full(&self.viewport, gtk::SelectionMode::Single);
        map.add_layer(&self.marker_layer);

        nearby
            .into_iter()
            // Select nearby cell items and drop any Nones
            .filter_map(|cell_index| self.cells.get(&cell_index))
            // We must sort items before adding to layer so they are added in a consistent order.
            // This prevents overlapping thumbnails from changing their order and flickering
            // when the map is dragged.
            .sorted_by_key(|x| x.visual.ordering_ts)
            .for_each(|item| {
                let widget = self.to_pin_thumbnail(&item.visual, Some(item.count), sender);

                let marker = shumate::Marker::builder().child(&widget).build();

                // Hmmm... using the cell_index lat/lng can put thumbnails kinda far from where
                // they occurred
                //let location: LatLng = cell_index.into();
                let Some(location) = item.visual.location else {
                    error!("Empty location after grouping");
                    return;
                };

                marker.set_location(location.lat(), location.lng());

                self.marker_layer.add_marker(&marker);
            });

        info!("{} cells on map", self.cells.len());
        info!(
            "{} thumbnails added to the map",
            self.marker_layer.markers().len()
        );
    }

    fn refresh(&mut self, sender: &ComponentSender<Self>) {
        let data = self.state.read().clone();
        let data = data.iter().filter(|x| x.location.is_some()).collect_vec();

        info!("{} items with location data", data.len());

        if let Some(most_recent) = data.iter().max_by(|x, y| x.ordering_ts.cmp(&y.ordering_ts)) {
            let location = most_recent.location.expect("must have location");
            info!("Centreing on most recent location at {}", location);
            let map = self.map.map().expect("must have map");
            map.center_on(location.lat(), location.lng());
        }

        self.viewport.set_zoom_level(DEFAULT_ZOOM_LEVEL);
        self.update_on_zoom(&PlacesAlbum::zoom_to_resolution(DEFAULT_ZOOM_LEVEL));
        self.update_on_move(sender);
        self.need_refresh = false;
    }

    /// Make thumbnail to put onto map
    fn to_pin_thumbnail(
        &self,
        visual: &Visual,
        count: Option<usize>,
        sender: &ComponentSender<PlacesAlbum>,
    ) -> gtk::AspectFrame {
        let thumbnail_path = self.thumbnailer
            .nearest_thumbnail(&visual.thumbnail_hash(), ThumbnailSize::Normal);

        let picture = if let Some(thumbnail_path) = thumbnail_path {
            let picture = gtk::Picture::for_filename(thumbnail_path);
            picture.set_content_fit(gtk::ContentFit::Cover);
            picture
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/app/fotema/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                WIDE_EDGE_LENGTH,
                WIDE_EDGE_LENGTH,
                true,
            )
            .unwrap();
            let texture = gdk::Texture::for_pixbuf(&pb);
            let picture = gtk::Picture::for_paintable(&texture);
            picture.set_content_fit(gtk::ContentFit::Fill);
            picture
        };

       let hclamp = adw::Clamp::builder()
            .maximum_size(100)
            .child(&picture)
            .orientation(gtk::Orientation::Horizontal)
            .build();

        let vclamp = adw::Clamp::builder()
            .maximum_size(100)
            .child(&hclamp)
            .orientation(gtk::Orientation::Vertical)
            .build();

        hclamp.add_write_only_binding(&self.edge_length, "maximum-size");
        vclamp.add_write_only_binding(&self.edge_length, "maximum-size");

        let frame = gtk::Frame::new(None);

        let count = count.unwrap_or(1);

        if count > 1 {
            // if there is a count then overlay the number in the bottom right corner.
            let label = gtk::Label::builder()
                .label(format!("{}", count))
                .css_classes(["map-thumbnail-label-text"])
                .build();

            let label_frame = gtk::Frame::builder()
                .halign(gtk::Align::End)
                .valign(gtk::Align::End)
                .css_classes(["map-thumbnail-label-frame"])
                .child(&label)
                .build();

            label_frame.set_margin_all(4);

            let overlay = gtk::Overlay::builder()
                .child(&vclamp)
                .build();

            overlay.add_overlay(&label_frame);

            frame.set_child(Some(&overlay));
        } else {
            frame.set_child(Some(&vclamp));
        }

        frame.add_css_class("map-thumbnail-border");

        let click = gtk::GestureClick::new();
        {
            let visual = visual.clone();
            let sender = sender.clone();
            let resolution = self.resolution;
            click.connect_released(move |_click, _, _, _| {
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

        let aframe = gtk::AspectFrame::builder()
            .obey_child(false)
            .ratio(1.0)
            .child(&frame)
            .build();

        aframe
    }
}
