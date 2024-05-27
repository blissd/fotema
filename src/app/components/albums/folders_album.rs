// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;

use fotema_core::visual::model::PictureOrientation;
use strum::IntoEnumIterator;

use itertools::Itertools;

use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::gdk_pixbuf;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;

use std::path;
use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashSet;

use crate::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;

use tracing::{event, Level};

const NARROW_EDGE_LENGTH: i32 = 170;
const WIDE_EDGE_LENGTH: i32 = 200;

#[derive(Debug)]
struct PhotoGridItem {
    folder_name: String,

    // Folder album cover
    picture: Arc<fotema_core::visual::Visual>,

    // Set of all thumbnails to allow for easy resizing on layout change.
    thumbnails: Rc<RefCell<HashSet<gtk::Picture>>>,
}

struct Widgets {
    picture: gtk::Picture,
    label: gtk::Label,
}
#[derive(Debug)]
pub enum FoldersAlbumInput {
    Activate,

    // Reload photos from database
    Refresh,

    Selected(u32), // Index into photo grid vector

    // Adapt to layout
    Adapt(adaptive::Layout),
}

#[derive(Debug)]
pub enum FoldersAlbumOutput {
    FolderSelected(path::PathBuf),
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
           my_box = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                gtk::AspectFrame {
                    gtk::Frame {
                        #[name(picture)]
                        gtk::Picture {
                            set_can_shrink: true,
                            set_width_request: NARROW_EDGE_LENGTH,
                            set_height_request: NARROW_EDGE_LENGTH,
                        }
                    }
                },

                #[name(label)]
                gtk::Label {
                    add_css_class: "caption-heading",
                    set_margin_top: 4,
                    set_margin_bottom: 12,
                },
            }
        }

        let widgets = Widgets { picture, label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets
            .label
            .set_text(format!("{}", self.folder_name).as_str());

        // Add our picture to the set of all pictures so it can be easily resized
        // when the window dimensions changes between wide and narrow.
        if !self.thumbnails.borrow().contains(&widgets.picture) {
            self.thumbnails.borrow_mut().insert(widgets.picture.clone());
        }

        if self.picture.thumbnail_path.as_ref().is_some_and(|x| x.exists()) {
            widgets
                .picture
                .set_filename(self.picture.thumbnail_path.clone());

            // Add CSS class for orientation
            let orientation = self.picture.thumbnail_orientation();
            widgets.picture.add_css_class(orientation.as_ref());
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/app/fotema/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                200, 200, true
            ).unwrap();
            let img = gdk::Texture::for_pixbuf(&pb);
            widgets.picture.set_paintable(Some(&img));
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.picture.set_filename(None::<&path::Path>);
        // clear orientation transformation css classes
        for orient in PictureOrientation::iter() {
            widgets.picture.remove_css_class(orient.as_ref());
        }
    }
}

pub struct FoldersAlbum {
    state: SharedState,
    active_view: ActiveView,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
    layout: adaptive::Layout,
    thumbnails: Rc<RefCell<HashSet<gtk::Picture>>>,
}

pub struct FoldersAlbumWidgets {
    // All pictures referenced by grid view.
    thumbnails: Rc<RefCell<HashSet<gtk::Picture>>>,
}

//#[relm4::component(pub)]
impl SimpleComponent for FoldersAlbum {
    type Init = (SharedState, ActiveView);
    type Input = FoldersAlbumInput;
    type Output = FoldersAlbumOutput;
    type Root = gtk::ScrolledWindow;
    type Widgets = FoldersAlbumWidgets;

    fn init_root() -> Self::Root {
        gtk::ScrolledWindow::builder()
            .vexpand(true)
            .build()
    }

    fn init(
        (state, active_view): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let grid_view = &photo_grid.view;
        grid_view.set_orientation(gtk::Orientation::Vertical);
        grid_view.set_single_click_activate(true);
        grid_view.connect_activate(move |_, idx| sender.input(FoldersAlbumInput::Selected(idx)));

        let model = FoldersAlbum {
            state,
            active_view,
            photo_grid,
            layout: adaptive::Layout::Narrow,
            thumbnails: Rc::new(RefCell::new(HashSet::new())),
        };

        let widgets = FoldersAlbumWidgets {
            thumbnails: model.thumbnails.clone(),
        };

        root.set_child(Some(&model.photo_grid.view));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
           FoldersAlbumInput::Activate => {
                *self.active_view.write() = ViewName::Folders;
                if self.photo_grid.is_empty() {
                    self.refresh();
                }
            },
            FoldersAlbumInput::Refresh => {
                if *self.active_view.read() == ViewName::Folders {
                    self.refresh();
                } else {
                    self.photo_grid.clear();
                }
            },
            FoldersAlbumInput::Selected(index) => {
                event!(Level::DEBUG, "Folder selected index: {}", index);
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let item = item.borrow();
                    event!(Level::DEBUG, "Folder selected item: {}", item.folder_name);

                    let _ = sender
                        .output(FoldersAlbumOutput::FolderSelected(item.picture.parent_path.clone()));
                }
            },
            FoldersAlbumInput::Adapt(layout) => {
                self.layout = layout;
            },
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        match self.layout {
            // Update thumbnail size depending on adaptive layout type
            adaptive::Layout::Narrow => {
                let pics = widgets.thumbnails.borrow_mut();
                for pic in pics.iter() {
                    pic.set_width_request(NARROW_EDGE_LENGTH);
                    pic.set_height_request(NARROW_EDGE_LENGTH);
                }
            },
            adaptive::Layout::Wide => {
                let pics = widgets.thumbnails.borrow_mut();
                for pic in pics.iter() {
                    pic.set_width_request(WIDE_EDGE_LENGTH);
                    pic.set_height_request(WIDE_EDGE_LENGTH);
                }
             },
         }
     }
}

impl FoldersAlbum {
    fn refresh(&mut self) {
        let all = {
            let data = self.state.read();
            data.clone()
                .into_iter()
                .sorted_by_key(|pic| pic.parent_path.clone())
                .chunk_by(|pic| pic.parent_path.clone())
        };

        let mut pictures = Vec::new();

        for (_key, mut group) in &all {
            let first = group.nth(0).expect("Groups can't be empty");
            let album = PhotoGridItem {
                folder_name: first.folder_name().unwrap_or("-".to_string()),
                picture: first.clone(),
                    thumbnails: self.thumbnails.clone(),
            };
            pictures.push(album);
        }

        pictures.sort_by_key(|pic| pic.folder_name.clone());

        self.thumbnails.borrow_mut().clear();
        self.photo_grid.clear();
        self.photo_grid.extend_from_iter(pictures);

        // NOTE folder view is not sorted by a timestamp, so don't scroll to end.
    }
}
