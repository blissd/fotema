// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use photos_core::repo::PictureId;
use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::*;
use std::sync::{Arc, Mutex};

use crate::app::components::photo_info::PhotoInfo;
use crate::app::components::photo_info::PhotoInfoInput;

#[derive(Debug)]
pub enum OnePhotoInput {
    ViewPhoto(PictureId),
    ToggleInfo,
}

#[derive(Debug)]
pub struct OnePhoto {
    repo: Arc<Mutex<photos_core::Repository>>,

    // Photo to show
    picture: gtk::Picture,

    // Info for photo
    photo_info: Controller<PhotoInfo>,

    // Photo and photo info views
    split_view: adw::OverlaySplitView,

    title: String,
}

#[relm4::component(pub)]
impl SimpleComponent for OnePhoto {
    type Init = (photos_core::Scanner, Arc<Mutex<photos_core::Repository>>);
    type Input = OnePhotoInput;
    type Output = ();

    view! {

        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar {
                #[wrap(Some)]
                set_title_widget = &gtk::Label {
                    #[watch]
                    set_label: model.title.as_ref(),
                    add_css_class: "title",
                },
                pack_end = &gtk::Button {
                    set_icon_name: "info-outline-symbolic",
                    connect_clicked => OnePhotoInput::ToggleInfo,
                }
            },

            #[wrap(Some)]
            #[local_ref]
            set_content = &split_view -> adw::OverlaySplitView {
                #[wrap(Some)]
                set_sidebar = model.photo_info.widget(),

                set_sidebar_position: gtk::PackType::End,

                #[wrap(Some)]
                #[local_ref]
                set_content = &picture -> gtk::Picture {
                    set_can_shrink: true,
                    set_valign: gtk::Align::Center,
                }
            }
        }
    }

    fn init(
        (scanner, repo): Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let picture = gtk::Picture::new();

        let photo_info = PhotoInfo::builder().launch(scanner).detach();

        let split_view = adw::OverlaySplitView::new();

        let model = OnePhoto {
            repo,
            picture: picture.clone(),
            photo_info,
            split_view: split_view.clone(),
            title: String::from("-"),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
     match msg {
            OnePhotoInput::ViewPhoto(picture_id) => {
                println!("Showing photo for {}", picture_id);
                let result = self.repo.lock().unwrap().get(picture_id);
                if let Ok(Some(pic)) = result {
                    self.title = pic.path
                        .file_name()
                        .map(|x| x.to_string_lossy().to_string())
                        .unwrap_or(String::from("-"));

                    self.picture.set_filename(Some(pic.path.clone()));
                    self.photo_info.emit(PhotoInfoInput::ShowInfo(pic.path));
                } else {
                    println!("Failed loading {}: {:?}", picture_id, result);
                }
            },
            OnePhotoInput::ToggleInfo => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_show_sidebar(!show);
            }
        }
    }
}
