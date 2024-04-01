// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::{ OrientableExt};
use photos_core::repo::PictureId;
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::*;
use std::sync::{Arc, Mutex};

use crate::app::components::photo_info::PhotoInfo;
use crate::app::components::photo_info::PhotoInfoInput;

#[derive(Debug)]
pub enum OnePhotoInput {
    ViewPhoto(PictureId),
}

#[derive(Debug)]
pub struct OnePhoto {
    repo: Arc<Mutex<photos_core::Repository>>,
    photo_info: Controller<PhotoInfo>,
    picture: gtk::Picture,

}

#[relm4::component(pub)]
impl SimpleComponent for OnePhoto {
    type Init = (photos_core::Scanner, Arc<Mutex<photos_core::Repository>>);
    type Input = OnePhotoInput;
    type Output = ();

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar,

            #[wrap(Some)]
            set_content = &adw::OverlaySplitView {
                #[wrap(Some)]
                set_sidebar = model.photo_info.widget(),

                set_sidebar_position: gtk::PackType::End,

                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    #[local_ref]
                    picture -> gtk::Picture {
                        set_can_shrink: true,
                        set_valign: gtk::Align::Center,
                    }
                }
            }
        }
    }

    fn init(
        (scanner, repo): Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let picture = gtk::Picture::new();

        let photo_info = PhotoInfo::builder().launch(scanner).detach();

        let model = OnePhoto {
            repo,
            picture: picture.clone(),
            photo_info,
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
                    self.picture.set_filename(Some(pic.path.clone()));
                    self.photo_info.emit(PhotoInfoInput::ShowInfo(pic.path));
                } else {
                    println!("Failed loading {}: {:?}", picture_id, result);
                }
            }
        }
    }
}
