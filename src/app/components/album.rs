// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use fotema_core::VisualId;
use fotema_core::YearMonth;
use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::prelude::*;
use relm4::gtk::gdk_pixbuf;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug)]
pub enum AlbumFilter {
    // Show no photos
    None,

    // Show all photos
    All,

    // Show only selfies
    Selfies,

    // Show only videos
    Videos,

    // Show only motion photos (live photos)
    Motion,

    // Show photos only for folder
    Folder(PathBuf),
}

#[derive(Debug)]
pub enum AlbumInput {
    /// User has selected photo in grid view
    Selected(u32), // Index into a Vec

    // Scroll to first photo of year/month.
    GoToMonth(YearMonth),

    // Reload photos from database
    Refresh,

    // I'd like to pass a closure of Fn(Picture)->bool for the filter... but Rust
    // is making that too hard.

    // Show no photos
    Filter(AlbumFilter),
}

#[derive(Debug)]
pub enum AlbumOutput {
    /// User has selected photo or video in grid view
    Selected(VisualId),
}

#[derive(Debug)]
struct PhotoGridItem {
    visual: Arc<fotema_core::visual::Visual>,
//    photo_thumbnailer: photo::Thumbnailer,
}

struct PhotoGridItemWidgets {
    picture: gtk::Picture,
    status_overlay: gtk::Frame,
    motion_type_icon: gtk::Image,
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
        };

        (root, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
    /*
        if self.visual.thumbnail_path.as_ref().is_none() && self.visual.picture_id.is_some() {
            let visual = self.visual.borrow_mut();
            let thumbnail_path = self.photo_thumbnailer.thumbnail(&visual.picture_id, visual.picture_path.expect("must have picture path"));
        }
        */

        if self.visual.thumbnail_path.as_ref().is_some_and(|x| x.exists()) {
            widgets.picture.set_filename(self.visual.thumbnail_path.clone());
            if self.visual.is_motion_photo() || self.visual.is_video_only() {
                widgets.status_overlay.set_visible(true);
                if self.visual.is_video_only() {
                    widgets.motion_type_icon.set_icon_name(Some("play-symbolic"));
                } else if self.visual.is_motion_photo() {
                    widgets.motion_type_icon.set_icon_name(Some("cd-symbolic"));
                }
            } else {
                widgets.status_overlay.set_visible(false);
            }
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/dev/romantics/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                200, 200, true
            ).unwrap();
           let img = gdk::Texture::for_pixbuf(&pb);
            widgets.picture.set_paintable(Some(&img));
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&Path>);
        widgets.motion_type_icon.set_icon_name(None);
        widgets.status_overlay.set_visible(false);
    }
}

pub struct Album {
    repo: fotema_core::visual::Library,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
}

#[relm4::component(pub)]
impl SimpleComponent for Album {
    type Init = (fotema_core::visual::Library, AlbumFilter);
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
        (repo, initial_filter): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let mut model = Album { repo, photo_grid };

        model.update_filter(initial_filter);

        let grid_view = &model.photo_grid.view;

        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            AlbumInput::Filter(filter) => {
                self.update_filter(filter);
            }
            AlbumInput::Refresh => {
                let all = self
                    .repo
                    .all()
                    .into_iter()
                    .map(|visual| PhotoGridItem { visual });

                self.photo_grid.clear();

                //self.photo_grid.add_filter(move |item| (self.photo_grid_filter)(&item.picture));
                self.photo_grid.extend_from_iter(all.into_iter());

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
            AlbumInput::Selected(index) => {
                // Photos are filters so must use get_visible(...) over get(...), otherwise
                // wrong photo is displayed.
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let visual_id = item.borrow().visual.visual_id.clone();
                    println!("index {} has visual_id {}", index, visual_id);
                    let result = sender.output(AlbumOutput::Selected(visual_id));
                    println!("Result = {:?}", result);
                }
            }
            AlbumInput::GoToMonth(ym) => {
                println!("Showing for month: {}", ym);
                let index_opt = self.photo_grid.find(|p| p.visual.year_month() == ym);
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

impl Album {
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

    fn update_filter(&mut self, filter: AlbumFilter) {
        self.photo_grid.clear_filters();
        //self.photo_grid.add_filter(Album::filter_has_thumbnail);

        match filter {
            AlbumFilter::All => {
                // If there are no filters, then all photos will be displayed.
            }
            AlbumFilter::None => {
                self.photo_grid.add_filter(Album::filter_none);
            }
            AlbumFilter::Videos => {
                self.photo_grid.add_filter(Album::filter_videos);
            }
            AlbumFilter::Motion => {
                self.photo_grid.add_filter(Album::filter_motion_photos);
            }
            AlbumFilter::Selfies => {
                self.photo_grid.add_filter(Album::filter_selfie);
            }
            AlbumFilter::Folder(path) => {
                self.photo_grid.add_filter(Album::filter_folder(path));
            }
        }
    }

    fn filter_none(_item: &PhotoGridItem) -> bool {
        false
    }

    fn filter_selfie(item: &PhotoGridItem) -> bool {
        item.visual.is_selfie()
    }

    fn filter_videos(item: &PhotoGridItem) -> bool {
        item.visual.is_video_only()
    }

    fn filter_motion_photos(item: &PhotoGridItem) -> bool {
        item.visual.is_motion_photo()
    }

    fn filter_folder(path: PathBuf) -> impl Fn(&PhotoGridItem) -> bool {
        move |item: &PhotoGridItem| item.visual.parent_path  == path
    }
}
