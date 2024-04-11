// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{adw, ComponentParts, ComponentSender, SimpleComponent};
use relm4::adw::prelude::AdwDialogExt;
use relm4::gtk::prelude::WidgetExt;


pub struct PreferencesDialog {
    parent: adw::ApplicationWindow,
}

impl SimpleComponent for PreferencesDialog {
    type Init = adw::ApplicationWindow;
    type Widgets = adw::PreferencesDialog;
    type Input = ();
    type Output = ();
    type Root = adw::PreferencesDialog;

    fn init_root() -> Self::Root {
        adw::PreferencesDialog::builder()
            .build()
    }

    fn init(
        parent: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {parent};

        let widgets = root.clone();

        ComponentParts { model, widgets }
    }

    fn update_view(&self, dialog: &mut Self::Widgets, _sender: ComponentSender<Self>) {
       dialog.present(&self.parent);
    }
}
