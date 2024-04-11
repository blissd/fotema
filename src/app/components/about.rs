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
        adw::AboutDialog::builder()
            .application_icon(APP_ID)
            .license_type(gtk::License::Gpl30)
            .website("https://github.com/blissd/fotema")
            .issue_url("https://github.com/blissd/fotema/issues")
            .application_name("Fotema")
            .version(VERSION)
            //.translator_credits("translator-credits")
            .copyright("© 2024 David Bliss")
            .developers(vec!["David Bliss"])
            .designers(vec!["David Bliss"])
            .can_close(true)
            .build()
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
