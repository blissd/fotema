// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::VisualId;
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
    ViewPhoto(VisualId),
    ToggleInfo,

    // The photo/video page has been hidden so any playing media should stop.
    Hidden,
}

pub struct OnePhoto {
    library: fotema_core::Library,

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
    type Init = (fotema_core::Library, Controller<PhotoInfo>);
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
        (library, photo_info): Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let picture = gtk::Picture::new();

        let split_view = adw::OverlaySplitView::new();

        let model = OnePhoto {
            library,
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
            OnePhotoInput::Hidden => {
                self.picture.set_paintable(None::<&gdk::Paintable>);
                self.title = String::from("-");
            },
            OnePhotoInput::ViewPhoto(visual_id) => {
                println!("Showing item for {}", visual_id);
                let result = self.library.get(visual_id);

                let visual = if let Some(v) = result {
                    v
                } else {
                    println!("Failed loading visual item: {:?}", result);
                    return;
                };

                let visual_path = visual.picture_path.clone()
                    .or_else(|| visual.video_path.clone())
                    .expect("Must have path");

                self.title = visual_path.file_name()
                    .map(|x| x.to_string_lossy().to_string())
                    .unwrap_or(String::from("-"));

                self.picture.set_paintable(None::<&gdk::Paintable>);

                if visual.is_photo_only() {
                    let file = gio::File::for_path(visual_path.clone());
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
                    self.photo_info.emit(PhotoInfoInput::Photo(visual_id, image.info().clone()));
                } else { // video or motion photo
                    self.photo_info.emit(PhotoInfoInput::Video(visual_id));

                    let media_file = gtk::MediaFile::for_filename(visual.video_path.clone().expect("Must have video"));
                    self.picture.set_paintable(Some(&media_file));

                    if visual.is_motion_photo() {
                       media_file.set_muted(true);
                       media_file.set_loop(true);
                    } else {
                       media_file.set_muted(false);
                       media_file.set_loop(false);
                    }

                    media_file.play();
                }
            },
            OnePhotoInput::ToggleInfo => {
                let show = self.split_view.shows_sidebar();
                self.split_view.set_show_sidebar(!show);
            }
        }
    }
}
