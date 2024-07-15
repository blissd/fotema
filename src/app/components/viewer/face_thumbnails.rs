// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::gtk;
use relm4::adw::gdk;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;
use crate::fl;
use fotema_core::people;
use fotema_core::PictureId;

use tracing::{debug, info};


#[derive(Debug)]
pub enum FaceThumbnailsInput {
    // View an item.
    View(PictureId),

    // The photo/video page has been hidden so any playing media should stop.
    Hide,
}

#[derive(Debug)]
pub enum FaceThumbnailsOutput {

}

pub struct FaceThumbnails {
    people_repo: people::Repository,

    face_thumbnails: gtk::Box,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for FaceThumbnails {
    type Init = people::Repository;
    type Input = FaceThumbnailsInput;
    type Output = FaceThumbnailsOutput;

    view! {
        #[name(face_thumbnails)]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
        }
    }

    async fn init(
        people_repo: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let widgets = view_output!();

/*
        let face_thumbnails = gtk::Box::builder()
            .orientation(gtk::Orientation::Horizontal)
            .spacing(8)
            .build();
*/
        let model = Self {
            people_repo,
            face_thumbnails: widgets.face_thumbnails.clone(),
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, _sender: AsyncComponentSender<Self>) {
        match msg {
            FaceThumbnailsInput::Hide => {
                self.face_thumbnails.remove_all();
            },
            FaceThumbnailsInput::View(picture_id) => {
                info!("Showing faces for {}", picture_id);

                self.face_thumbnails.remove_all();

                if let Ok(faces) = self.people_repo.find_faces(&picture_id) {
                    debug!("Found {} faces", faces.len());
                    faces.into_iter()
                        .filter(|face| face.thumbnail_path.exists())
                        .for_each(|face| {
                            let thumbnail = gtk::Picture::for_filename(&face.thumbnail_path);
                            thumbnail.set_content_fit(gtk::ContentFit::ScaleDown);
                            thumbnail.set_width_request(50);
                            thumbnail.set_height_request(50);
                            //thumbnail.set_css_classes(&["face-small"]);

                            let frame = gtk::Frame::new(None);
                            frame.set_child(Some(&thumbnail));
                            frame.add_css_class("face-small");

                            self.face_thumbnails.append(&frame);
                        });
                }
            },
        }
    }
}
