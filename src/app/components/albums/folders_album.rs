// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;

use fotema_core::visual::model::PictureOrientation;
use fotema_core::thumbnailify::{Thumbnailer, ThumbnailSize};

use strum::IntoEnumIterator;

use itertools::Itertools;

use relm4::binding::*;
use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::gdk_pixbuf;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;

use std::path;
use std::sync::Arc;
use std::rc::Rc;

use crate::adaptive;
use crate::app::ActiveView;
use crate::app::SharedState;
use crate::app::ViewName;

use tracing::{Level, event, info};

const NARROW_EDGE_LENGTH: i32 = 170;
const WIDE_EDGE_LENGTH: i32 = 200;

#[derive(Debug)]
struct PhotoGridItem {
    folder_name: String,

    // Folder album cover
    visual: Arc<fotema_core::visual::Visual>,

    // Length of thumbnail edge to allow for resizing when layout changes.
    edge_length: I32Binding,

    thumbnailer: Rc<Thumbnailer>,
}

struct Widgets {
    picture: gtk::Picture,
    label: gtk::Label,

    // If the gtk::Picture has been bound to edge_length.
    is_bound: bool,
}
#[derive(Debug)]
pub enum FoldersAlbumInput {
    Activate,

    // Reload photos from database
    Refresh,

    FolderSelected(u32), // Index into photo grid vector

    // Adapt to layout
    Adapt(adaptive::Layout),

    /// No-op. After refreshing the thumbnail grid, the screen would be blank and thumbnails
    /// would not appear until clicking to another view and back. I don't know why this happens,
    /// and have only observed this behaviour on the folders album view. As a work around, send
    /// a no-op message to trigger a view redraw.
    Noop,
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
                gtk::Frame {
                    #[name(picture)]
                    gtk::Picture {
                        set_content_fit: gtk::ContentFit::Cover,
                        set_width_request: NARROW_EDGE_LENGTH,
                        set_height_request: NARROW_EDGE_LENGTH,
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

        let widgets = Widgets {
            picture,
            label,
            is_bound: false,
        };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.label.set_text(&self.folder_name.to_string());

        // If we repeatedly bind, then Fotema will die with the following error:
        // (fotema:2): GLib-GObject-CRITICAL **: 13:26:14.297: Too many GWeakRef registered
        // GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        // Bail out! GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        if !widgets.is_bound {
            widgets
                .picture
                .add_write_only_binding(&self.edge_length, "width-request");
            widgets
                .picture
                .add_write_only_binding(&self.edge_length, "height-request");
            widgets.is_bound = true;
        }

        let thumbnail_path = self.thumbnailer
            .nearest_thumbnail(&self.visual.thumbnail_hash(), ThumbnailSize::Normal);

        if thumbnail_path.is_some() {
            widgets
                .picture
                .set_filename(thumbnail_path);
        } else {
            let pb = gdk_pixbuf::Pixbuf::from_resource_at_scale(
                "/app/fotema/Fotema/icons/scalable/actions/image-missing-symbolic.svg",
                200,
                200,
                true,
            )
            .unwrap();
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
    thumbnailer: Rc<Thumbnailer>,
}

#[relm4::component(pub)]
impl SimpleComponent for FoldersAlbum {
    type Init = (SharedState, ActiveView, Rc<Thumbnailer>);
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
        (state, active_view, thumbnailer): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let model = FoldersAlbum {
            state,
            active_view,
            photo_grid,
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
            thumbnailer,
        };

        let pictures_box = &model.photo_grid.view;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            FoldersAlbumInput::Noop => {
                info!("No-op received... so doing nothing. As expected :-/");
            }
            FoldersAlbumInput::Activate => {
                *self.active_view.write() = ViewName::Folders;
                if self.photo_grid.is_empty() {
                    self.refresh();
                }
            }
            FoldersAlbumInput::Refresh => {
                if *self.active_view.read() == ViewName::Folders {
                    info!("Folders view is active so refreshing");
                    self.refresh();

                    // Work-around to make grid appear.
                    sender.input(FoldersAlbumInput::Noop);
                } else {
                    info!("Folders view is inactive so clearing");
                    self.photo_grid.clear();
                }
            }
            FoldersAlbumInput::FolderSelected(index) => {
                event!(Level::DEBUG, "Folder selected index: {}", index);
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let item = item.borrow();
                    event!(Level::DEBUG, "Folder selected item: {}", item.folder_name);

                    let _ = sender.output(FoldersAlbumOutput::FolderSelected(
                        item.visual.parent_path.clone(),
                    ));
                }
            }
            FoldersAlbumInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            }
            FoldersAlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            }
        }
    }
}

impl FoldersAlbum {
    fn refresh(&mut self) {
        let all = {
            let data = self.state.read();
            data.clone()
                .into_iter()
                .sorted_by_key(|visual| visual.parent_path.clone())
                .chunk_by(|visual| visual.parent_path.clone())
        };

        let mut pictures = Vec::new();

        for (_key, mut group) in &all {
            let first = group.nth(0).expect("Groups can't be empty");
            let album = PhotoGridItem {
                folder_name: first.folder_name().unwrap_or("-".to_string()),
                visual: first.clone(),
                edge_length: self.edge_length.clone(),
                thumbnailer: self.thumbnailer.clone(),
            };
            pictures.push(album);
        }

        pictures.sort_by_key(|pic| pic.folder_name.clone());

        self.photo_grid.clear();
        self.photo_grid.extend_from_iter(pictures);

        // NOTE folder view is not sorted by a timestamp, so don't scroll to end.
    }
}
