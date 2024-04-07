// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
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
    type Root = gtk::Picture;
    type Widgets = ();

    fn setup(_item: &gtk::ListItem) -> (Self::Root, Self::Widgets) {
        relm4::view! {
            picture = gtk::Picture {
                set_can_shrink: true,
                set_valign: gtk::Align::Center,
                set_width_request: 200,
                set_height_request: 200,
            }
        }

        (picture, ())
    }

    fn bind(&mut self, _widgets: &mut Self::Widgets, picture: &mut Self::Root) {
        if self.picture.square_preview_path.as_ref().is_some_and(|f|f.exists()) {
            picture
                .set_filename(self.picture.square_preview_path.clone());
        } else {
            picture
                .set_resource(Some("/dev/romantics/Photos/icons/image-missing-symbolic.svg"));
        }
    }

    fn unbind(&mut self, _widgets: &mut Self::Widgets, picture: &mut Self::Root) {
        picture.set_filename(None::<&Path>);
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
        gtk::ScrolledWindow {
            set_vexpand: true,

            #[local_ref]
            grid_view -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,
                //set_max_columns: 3,

                connect_activate[sender] => move |_, idx| {
                    sender.input(AlbumInput::PhotoSelected(idx))
                },
            }
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

                    // We must scroll to a valid index... but we can't get the index of the
                    // last item if filters are enabled. So as a workaround disable filters,
                    // scroll to end, and then enable filters.

                    self.disable_filters();

                    self.photo_grid.view
                        .scroll_to(self.photo_grid.len() - 1, gtk::ListScrollFlags::SELECT, None);

                    self.enable_filters();
                }
            },
            AlbumInput::PhotoSelected(index) => {
                // Photos are filters so must use get_visible(...) over get(...), otherwise
                // wrong photo is displayed.
                if let Some(item) = self.photo_grid.get_visible(index) {
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
                // This doesn't really work properly.
                // The intent was to have updated thumbnails be immediately visible, but I've
                // never been able to make this work in a was that is fast and delightful.
                // If the grid item thumbnail path is updated, then the item isn't refreshed
                // visually so the thumbnail isn't visible until there is a mouseover event, which
                // makes the app seem like it isn't working correctly.
                // I've tried triggering a refresh with adding and removing items, and with
                // disabling and enabling filters, but both have problems. Adding and removing
                // items is slow and seems to get slower as more previews are generated.
                // Disabling and enabling filters looked promising, but causing a lot of jumping
                // around and also resulted in a large slowdown that prompted "Force Quit" dialogs
                // to pop up.
                //
                // So, for now this code is unused and will have to be removed if a decent
                // working solution can't be found.
/*
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
                */
            },
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
        self.photo_grid.add_filter(Album::filter_has_thumbnail);

        match filter {
            AlbumFilter::All => {
                // If there are no filters, then all photos will be displayed.
            },
            AlbumFilter::None => {
                self.photo_grid.add_filter(Album::filter_none);
            },
            AlbumFilter::Selfies => {
                self.photo_grid.add_filter(Album::filter_selfie);
            },
            AlbumFilter::Folder(path) => {
                self.photo_grid.add_filter(Album::filter_folder(path));
            },
        }
    }

    fn filter_none(_item: &PhotoGridItem) -> bool {
        false
    }

    fn filter_selfie(item: &PhotoGridItem) -> bool {
        item.picture.is_selfie
    }

    fn filter_folder(path: PathBuf) -> impl Fn(&PhotoGridItem) -> bool {
        move |item: &PhotoGridItem| item.picture.parent_path().is_some_and(|p| p == path)
    }

    fn filter_has_thumbnail(item: &PhotoGridItem) -> bool {
        item.picture.square_preview_path.as_ref().is_some_and(|p| p.exists())
    }
}
