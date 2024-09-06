// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use ashpd::{
    desktop::file_chooser::OpenFileRequest,
    WindowIdentifier,
};

use relm4::gtk;
use relm4::gtk::prelude::*;
use relm4::*;
use relm4::prelude::*;

use crate::fl;

use tracing::{error, info};

use std::path::PathBuf;

#[derive(Debug)]
pub enum OnboardInput {

    /// Button to choose file dialog has been clicked.
    ChooseDirectory,
}

#[derive(Debug)]
pub enum OnboardOutput {
    /// Onboarding process is complete
    Done(PathBuf),
}

pub struct Onboard {
    root: adw::StatusPage,
}

#[relm4::component(pub async)]
impl SimpleAsyncComponent for Onboard {
    type Init = ();
    type Input = OnboardInput;
    type Output = OnboardOutput;

    view! {
        adw::StatusPage {
            set_valign: gtk::Align::Start,
            set_vexpand: true,

            set_icon_name: Some("image-missing-symbolic"),
            set_title: &fl!("onboard-select-pictures", "title"),
            set_description: Some(&fl!("onboard-select-pictures", "description")),

            #[wrap(Some)]
            set_child = &adw::Clamp {
                set_orientation: gtk::Orientation::Horizontal,
                set_maximum_size: 360,

                #[wrap(Some)]
                set_child = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    //#[local_ref]
                    gtk::Button {
                        set_label: &fl!("onboard-select-pictures", "button"),
                        add_css_class: "suggested-action",
                        add_css_class: "pill",
                        connect_clicked => OnboardInput::ChooseDirectory,
                    },

                    //model.transcode_progress.widget(),
                }
            }
        },
    }

    async fn init(
        _: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {

        let widgets = view_output!();

        let model = Onboard {root};

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            OnboardInput::ChooseDirectory => {
                info!("Presenting directory chooser");
                if let Some(root) = gtk::Widget::root(self.root.widget_ref()) {
                    let identifier = WindowIdentifier::from_native(&root).await;
                    let request = OpenFileRequest::default()
                        .directory(true)
                        .identifier(identifier)
                        .modal(true) // can't be modal without identifier.
                        .multiple(true);

                    match request.send().await.and_then(|r| r.response()) {
                        Ok(files) => {
                            info!("Open: {:?}", files);
                            if let Some(first) = files.uris().first() {
                                info!("User has chosen picture library at: {:?}", first.path());
                                let _ = sender.output(OnboardOutput::Done(PathBuf::from(first.path())));
                            }
                        }
                        Err(err) => {
                            error!("Failed to open a file: {err}");
                        }
                    }
                 }
            },

        }
    }
}
