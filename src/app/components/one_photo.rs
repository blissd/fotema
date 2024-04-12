// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use photos_core::photo::repo::PictureId;
use relm4::gtk;
use relm4::adw::gdk;
use relm4::gtk::gio;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;
use glycin;

use crate::app::components::photo_info::PhotoInfo;
use crate::app::components::photo_info::PhotoInfoInput;

#[derive(Debug)]
pub enum OnePhotoInput {
    ViewPhoto(PictureId),
    ToggleInfo,
}

#[derive(Debug)]
pub struct OnePhoto {
    repo: photos_core::photo::Repository,

    // Photo to show
    picture: gtk::Picture,

    // Info for photo
    photo_info: Controller<PhotoInfo>,

    // Photo and photo info views
    split_view: adw::OverlaySplitView,

    title: String,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for OnePhoto {
    type Init = (photos_core::photo::Scanner, photos_core::photo::repo::Repository);
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
                    //set_vexpand: true,
                    //set_hexpand: true,
                    //set_can_shrink: true,
                    //set_valign: gtk::Align::Center,
                }
            }
        }
    }

    async fn init(
        (scanner, repo): Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

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

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
            OnePhotoInput::ViewPhoto(picture_id) => {
                println!("Showing photo for {}", picture_id);
                let result = self.repo.get(picture_id);
                if let Ok(Some(pic)) = result {
                    self.title = pic.path
                        .file_name()
                        .map(|x| x.to_string_lossy().to_string())
                        .unwrap_or(String::from("-"));

                    self.picture.set_paintable(None::<&gdk::Paintable>);

                    let file = gio::File::for_path(pic.path.clone());
                    let image_result = glycin::Loader::new(file).load().await;

                    let image = if let Ok(image) = image_result {
                        image
                    } else {
                        println!("Failed loading image: {:?}", image_result);
                        return;
                    };

                    let frame = if let Ok(frame) = image.next_frame().await {
                        frame
                    } else {
                        println!("Failed getting image frame");
                        return;
                    };

                    let texture = frame.texture;

                    self.picture.set_paintable(Some(&texture));
                    self.photo_info.emit(PhotoInfoInput::ShowInfo(pic.path));

                    //if i

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
