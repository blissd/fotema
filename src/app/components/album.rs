// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::{BoxExt, OrientableExt};
use photos_core;
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use relm4::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use photos_core::YearMonth;
use photos_core::repo::PictureId;

#[derive(Debug)]
struct PhotoGridItem {
    picture: photos_core::repo::Picture,
    //square_preview: Option<gio::File>,
}

struct Widgets {
    picture: gtk::Picture,
}

#[derive(Debug)]
pub enum AlbumFilter {
    // Show no photos
    None,

    // Show all photos
    All,

    // Show only selfies
    Selfies,

    // Show photos only for folder
    Folder(PathBuf),
}

#[derive(Debug)]
pub enum AlbumInput {
    /// User has selected photo in grid view
    PhotoSelected(u32), // Index into a Vec

    // Scroll to first photo of year/month.
    GoToMonth(YearMonth),

    // Preview has been updated
    PreviewUpdated(PictureId, Option<PathBuf>),

    // Reload photos from database
    Refresh,

    // I'd like to pass a closure of Fn(Picture)->bool for the filter... but Rust
    // is making that too hard.

    // Show no photos
    Filter(AlbumFilter),
}

#[derive(Debug)]
pub enum AlbumOutput {
    /// User has selected photo in grid view
    PhotoSelected(PictureId),
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
           my_box = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 1,

                #[name = "picture"]
                gtk::Picture {
                    set_can_shrink: true,
                    set_valign: gtk::Align::Center,
                    set_width_request: 200,
                    set_height_request: 200,
                }
            }
        }

        let widgets = Widgets { picture };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        if self.picture.square_preview_path.as_ref().is_some_and(|f|f.exists()) {
            widgets
                .picture
                .set_filename(self.picture.square_preview_path.clone());
        } else {
            widgets
                .picture
                .set_resource(Some("/dev/romantics/Photos/icons/image-missing-symbolic.svg"));
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&Path>);
    }
}

pub struct Album {
    repo: Arc<Mutex<photos_core::Repository>>,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for Album {
    type Init = (Arc<Mutex<photos_core::Repository>>, AlbumFilter);
    type Input = AlbumInput;
    type Output = AlbumOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 0,
            set_margin_all: 0,

            gtk::ScrolledWindow {

                //set_propagate_natural_height: true,
                //set_has_frame: true,
                set_vexpand: true,

                #[local_ref]
                grid_view -> gtk::GridView {
                    set_orientation: gtk::Orientation::Vertical,
                    set_single_click_activate: true,
                    //set_max_columns: 3,

                    connect_activate[sender] => move |_, idx| {
                        sender.input(AlbumInput::PhotoSelected(idx))
                    },
                },
            },
        }
    }

    async fn init(
        (repo, initial_filter): Self::Init,
        _root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {

        let photo_grid = TypedGridView::new();

        let mut model = Album {
            repo,
            photo_grid,
        };

        model.update_filter(initial_filter);

        let grid_view = &model.photo_grid.view;

        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            AlbumInput::Filter(filter) => {
                self.update_filter(filter);
            },
            AlbumInput::Refresh => {
                let all_pictures = self.repo
                    .lock()
                    .unwrap()
                    .all()
                    .unwrap()
                    .into_iter()
                    .map(|picture| PhotoGridItem {
                        picture,
                    });

                self.photo_grid.clear();

                //self.photo_grid.add_filter(move |item| (self.photo_grid_filter)(&item.picture));
                self.photo_grid.extend_from_iter(all_pictures.into_iter());

                if !self.photo_grid.is_empty(){
                    self.photo_grid.view
                        .scroll_to(self.photo_grid.len() - 1, gtk::ListScrollFlags::SELECT, None);
                }
            },
            AlbumInput::PhotoSelected(index) => {
                if let Some(item) = self.photo_grid.get(index) {
                    let picture_id = item.borrow().picture.picture_id;
                    println!("index {} has picture_id {}", index, picture_id);
                    let result = sender.output(AlbumOutput::PhotoSelected(picture_id));
                    println!("Result = {:?}", result);
                }
            },
            AlbumInput::GoToMonth(ym) => {
                println!("Showing for month: {}", ym);
                let index_opt = self.photo_grid.find(|p| p.picture.year_month() == ym);
                println!("Found: {:?}", index_opt);
                if let Some(index) = index_opt {
                    let flags = gtk::ListScrollFlags::SELECT;
                    println!("Scrolling to {}", index);
                    self.photo_grid.view.scroll_to(index, flags, None);
                }
            },
            AlbumInput::PreviewUpdated(id, path) => {
                println!("Preview updated ");

                if self.photo_grid.is_empty() {
                    // WARN calling find() on an empty grid causes a crash without
                    // a stack trace :-/
                    return;
                }

                let Some(index) = self.photo_grid.find(|p| p.picture.picture_id == id) else {
                    println!("No index for picture id: {}", id);
                    return;
                };

                let Some(item) = self.photo_grid.get(index) else {
                    return;
                };

                {
                    let mut item = item.borrow_mut();
                    item.picture.square_preview_path = path;
                }
            },
        }
    }
}

impl Album {

    fn update_filter(&mut self, filter: AlbumFilter) {
        match filter {
            AlbumFilter::None => {
                self.photo_grid.clear_filters();
                self.photo_grid.add_filter(Album::filter_none);
            },
            AlbumFilter::All => {
                self.photo_grid.clear_filters();
                //self.photo_grid.add_filter(Album::filter_all);
            },
            AlbumFilter::Selfies => {
                self.photo_grid.clear_filters();
                self.photo_grid.add_filter(Album::filter_selfie);
            },
            AlbumFilter::Folder(path) => {
                self.photo_grid.clear_filters();
                self.photo_grid.add_filter(Album::filter_folder(path));
            },
        }
    }

    fn filter_all(item: &PhotoGridItem) -> bool {
        true
    }

    fn filter_none(item: &PhotoGridItem) -> bool {
        false
    }

    fn filter_selfie(item: &PhotoGridItem) -> bool {
        item.picture.is_selfie
    }

    fn filter_folder(path: PathBuf) -> impl Fn(&PhotoGridItem) -> bool {
        move |item: &PhotoGridItem| item.picture.parent_path().is_some_and(|p| p == path)
    }
}
