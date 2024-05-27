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
use relm4::binding::*;

use std::path;
use std::sync::Arc;

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

    // Length of thumbnail edge to allow for resizing when layout changes.
    edge_length: I32Binding,

    // If the gtk::Picture has been bound to edge_length.
    is_bound: bool,
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

    FolderSelected(u32), // Index into photo grid vector

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

        // If we repeatedly bind, then Fotema will die with the following error:
        // (fotema:2): GLib-GObject-CRITICAL **: 13:26:14.297: Too many GWeakRef registered
        // GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        // Bail out! GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        if !self.is_bound {
            widgets.picture.add_write_only_binding(&self.edge_length, "width-request");
            widgets.picture.add_write_only_binding(&self.edge_length, "height-request");
            self.is_bound = true;
        }

        if self.picture.thumbnail_path.as_ref().is_some_and(|x| x.exists())
        {
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
    edge_length: I32Binding,
}

#[relm4::component(pub)]
impl SimpleComponent for FoldersAlbum {
    type Init = (SharedState, ActiveView);
    type Input = FoldersAlbumInput;
    type Output = FoldersAlbumOutput;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,

            #[local_ref]
            pictures_box -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,

                connect_activate[sender] => move |_, idx| {
                    sender.input(FoldersAlbumInput::FolderSelected(idx))
                }
            }
        }
    }

    fn init(
        (state, active_view): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let model = FoldersAlbum {
            state,
            active_view,
            photo_grid,
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
        };

        let pictures_box = &model.photo_grid.view;

        let widgets = view_output!();

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
            FoldersAlbumInput::FolderSelected(index) => {
                event!(Level::DEBUG, "Folder selected index: {}", index);
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let item = item.borrow();
                    event!(Level::DEBUG, "Folder selected item: {}", item.folder_name);

                    let _ = sender
                        .output(FoldersAlbumOutput::FolderSelected(item.picture.parent_path.clone()));
                }
            },
            FoldersAlbumInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            },
            FoldersAlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
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
               // .filter(|x| x.thumbnail_path.exists())
                .sorted_by_key(|pic| pic.parent_path.clone())
                .chunk_by(|pic| pic.parent_path.clone())
                //.collect::<Vec<Arc<Visual>>>()
        };

        let mut pictures = Vec::new();

        for (_key, mut group) in &all {
            let first = group.nth(0).expect("Groups can't be empty");
            let album = PhotoGridItem {
                folder_name: first.folder_name().unwrap_or("-".to_string()),
                picture: first.clone(),
                edge_length: self.edge_length.clone(),
                is_bound: false,
            };
            pictures.push(album);
        }

        pictures.sort_by_key(|pic| pic.folder_name.clone());

        self.photo_grid.clear();
        self.photo_grid.extend_from_iter(pictures.into_iter());

        // NOTE folder view is not sorted by a timestamp, so don't scroll to end.
    }
}
