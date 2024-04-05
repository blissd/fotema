// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::{BoxExt, OrientableExt};
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
}

#[derive(Debug)]
pub enum FolderPhotosOutput {
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
           my_box = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 1,

                #[name(label)]
                gtk::Label {
                    add_css_class: "caption-heading",
                },

                adw::Clamp {
                    set_maximum_size: 200,

                    gtk::Frame {

                        #[name(picture)]
                        gtk::Picture {
                            set_can_shrink: true,
                            set_valign: gtk::Align::Center,
                            set_width_request: 200,
                            set_height_request: 200,
                        }
                    }
                }
            }
        }

        let widgets = Widgets { picture, label };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets
            .label
            .set_label(format!("{}", self.folder_name).as_str());

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
    pictures_grid_view: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for FolderPhotos {
    type Init = Arc<Mutex<photos_core::Repository>>;
    type Input = FolderPhotosInput;
    type Output = FolderPhotosOutput;

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
                pictures_box -> gtk::GridView {
                    set_orientation: gtk::Orientation::Vertical,
                    set_single_click_activate: true,
                    //set_max_columns: 3,

                    //connect_activate[sender] => move |_, idx| {
                    //    sender.input(MonthPhotosInput::MonthSelected(idx))
                    //},
                },
            },
        }
    }

    async fn init(
        repo: Self::Init,
        _root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {

        let grid_view_wrapper: TypedGridView<PhotoGridItem, gtk::SingleSelection> =
            TypedGridView::new();

        let model = FolderPhotos {
            repo,
            pictures_grid_view: grid_view_wrapper,
        };

        let pictures_box = &model.pictures_grid_view.view;

        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
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
                    let first = group.nth(0).unwrap();
                    let album = PhotoGridItem {
                        folder_name: first.folder_name().unwrap_or("-".to_string()),
                        picture: first.clone(),
                    };
                    pictures.push(album);
                }

                self.pictures_grid_view.clear();
                self.pictures_grid_view.extend_from_iter(pictures.into_iter());

                if !self.pictures_grid_view.is_empty(){
                    self.pictures_grid_view.view
                        .scroll_to(self.pictures_grid_view.len() - 1, gtk::ListScrollFlags::SELECT, None);
                }
            },
        }
    }
}
