// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::adw::{self, prelude::*};
use relm4::gtk::{self, gio, gdk, gdk_pixbuf};
use relm4::*;
use relm4::prelude::*;
use relm4::actions::{RelmAction, RelmActionGroup};


use crate::fl;
use fotema_core::people;
use fotema_core::PictureId;
use fotema_core::FaceId;

use tracing::{debug, error, info};

use std::path::PathBuf;

#[derive(Debug)]
pub enum PersonSelectInput {
    /// Associate face with a person by name.
    SetPerson(FaceId, PathBuf),
}

#[derive(Debug)]
pub enum PersonSelectOutput {

}

pub struct PersonSelect {
    people_repo: people::Repository,
    face_thumbnail: adw::Avatar,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for PersonSelect {
    type Init = people::Repository;
    type Input = PersonSelectInput;
    type Output = PersonSelectOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 8,

            #[local_ref]
            face_thumbnail -> adw::Avatar,
        }
    }

    async fn init(
        people_repo: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self>  {

        let face_thumbnail = adw::Avatar::builder()
            .size(100)
            .show_initials(false)
            .build();

        let widgets = view_output!();

        let model = Self {
            people_repo,
            face_thumbnail,
        };

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            PersonSelectInput::SetPerson(face_id, thumbnail) => {
                println!("set person for face {}", face_id);

                let img = gdk::Texture::from_filename(&thumbnail).ok();
                self.face_thumbnail.set_custom_image(img.as_ref());
            },
        }
    }
}
