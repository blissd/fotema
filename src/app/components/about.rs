// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{adw, gtk, ComponentParts, ComponentSender, SimpleComponent};
use relm4::adw::prelude::AdwDialogExt;


use crate::config::{APP_ID, VERSION};

pub struct AboutDialog {
    parent: adw::ApplicationWindow,
    dialog: adw::AboutDialog,
}

impl SimpleComponent for AboutDialog {
    type Init = adw::ApplicationWindow;
    type Widgets = adw::AboutDialog;
    type Input = ();
    type Output = ();
    type Root = adw::AboutDialog;

    fn init_root() -> Self::Root {
        let about = adw::AboutDialog::builder()
            .application_icon(APP_ID)
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/blissd/fotema")
            .issue_url("https://github.com/blissd/fotema/issues")
            .application_name("Fotema")
            .version(VERSION)
            //.translator_credits("translator-credits")
            .copyright("Copyright © 2024 David Bliss")
            .developer_name("David Bliss")
            .developers(vec!["David Bliss"])
            .designers(vec!["David Bliss"])
            .can_close(true)
            .build();

        about.add_acknowledgement_section(Some("Open Source Projects"), &[
            "Relm 4 https://relm4.org/",
            "Glycin https://gitlab.gnome.org/sophie-h/glycin",
            "FFmpeg https://ffmpeg.org/",
            "libheif https://github.com/strukturag/libheif",
            "libde265 https://github.com/strukturag/libde265",
        ]);

        about.add_legal_section("FFmpeg", Some("Copyright © 2024 FFmpeg"), gtk::License::Gpl30, None);
        about.add_legal_section("libheif", Some("Copyright © 2017–2023 Dirk Farin"), gtk::License::Lgpl30, None);
        about.add_legal_section("libde265", Some("Copyright © 2017–2023 Dirk Farin"), gtk::License::Lgpl30, None);

        about
    }

    fn init(
        parent: Self::Init,
        dialog: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            parent,
            dialog: dialog.clone(),
        };

        let widgets = dialog;

        ComponentParts { model, widgets }
    }

    fn update_view(&self, _: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        self.dialog.present(&self.parent);
    }
}
