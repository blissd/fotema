// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;
use photos_core;

use itertools::Itertools;
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use relm4::prelude::*;

use std::path;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct PhotoGridItem {
    folder_name: String,

    // Folder album cover
    picture: photos_core::repo::Picture,
}

struct Widgets {
    picture: gtk::Picture,
    label: gtk::Label,
}
#[derive(Debug)]
pub enum FolderPhotosInput {
    // Reload photos from database
    Refresh,

    FolderSelected(u32), // Index into photo grid vector
}

#[derive(Debug)]
pub enum FolderPhotosOutput {
    FolderSelected(path::PathBuf),
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
           my_box = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                adw::Clamp {
                    set_maximum_size: 200,
                    set_orientation: gtk::Orientation::Horizontal,

                    gtk::Frame {
                        #[name(picture)]
                        gtk::Picture {
                            set_can_shrink: true,
                            set_valign: gtk::Align::Center,
                            set_width_request: 200,
                            set_height_request: 200,
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
        widgets.picture.set_filename(None::<&path::Path>);
    }
}

pub struct FolderPhotos {
    repo: Arc<Mutex<photos_core::Repository>>,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for FolderPhotos {
    type Init = Arc<Mutex<photos_core::Repository>>;
    type Input = FolderPhotosInput;
    type Output = FolderPhotosOutput;

    view! {
        gtk::ScrolledWindow {
            //set_propagate_natural_height: true,
            //set_has_frame: true,
            set_vexpand: true,

            #[local_ref]
            pictures_box -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,
                //set_max_columns: 3,

                connect_activate[sender] => move |_, idx| {
                    sender.input(FolderPhotosInput::FolderSelected(idx))
                }
            }
        }
    }

    async fn init(
        repo: Self::Init,
        _root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {

        let photo_grid = TypedGridView::new();

        let model = FolderPhotos {
            repo,
            photo_grid,
        };

        let pictures_box = &model.photo_grid.view;

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            FolderPhotosInput::FolderSelected(index) => {
                println!("Folder selected index: {}", index);
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let item = item.borrow();
                    println!("Folder selected item: {}", item.folder_name);

                    if let Some(folder_path) = item.picture.parent_path() {
                        let _ = sender.output(FolderPhotosOutput::FolderSelected(folder_path));
                    }
                }
            },
            FolderPhotosInput::Refresh => {

                let all_pictures = self.repo
                    .lock().unwrap()
                    .all()
                    .unwrap()
                    .into_iter()
                    .sorted_by_key(|pic| pic.parent_path())
                    .group_by(|pic| pic.parent_path());

                let mut pictures = Vec::new();

                for (_key, mut group) in &all_pictures {
                    let first = group.nth(0).expect("Groups can't be empty");
                    let album = PhotoGridItem {
                        folder_name: first.folder_name().unwrap_or("-".to_string()),
                        picture: first.clone(),
                    };
                    pictures.push(album);
                }

                pictures.sort_by_key(|pic| pic.folder_name.clone());

                self.photo_grid.clear();
                self.photo_grid.extend_from_iter(pictures.into_iter());

                // NOTE folder view is not sorted by a timestamp, so don't scroll to end.
            },
        }
    }
}
