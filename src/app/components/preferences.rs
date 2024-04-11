// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{adw, ComponentParts, ComponentSender, SimpleComponent};
use relm4::adw::prelude::AdwDialogExt;
use relm4::gtk::prelude::SettingsExt;
use relm4::gtk::gio;
use relm4::adw::prelude::PreferencesDialogExt;
use relm4::adw::prelude::PreferencesPageExt;
use relm4::adw::prelude::PreferencesGroupExt;
use relm4::adw::prelude::ActionRowExt;
use relm4::adw::prelude::PreferencesRowExt;

use crate::config::APP_ID;


pub struct PreferencesDialog {
    parent: adw::ApplicationWindow,
    dialog: adw::PreferencesDialog,

    // Preference values
    show_selfies: bool,
}

#[derive(Debug)]
pub enum PreferencesInput {
    Present,
    ShowSelfies(bool),
}

#[derive(Debug)]
pub enum PreferencesOutput {
    Updated,
}

#[relm4::component(pub)]
impl SimpleComponent for PreferencesDialog {
    type Init = adw::ApplicationWindow;
    type Input = PreferencesInput;
    type Output = PreferencesOutput;

    view!{
        adw::PreferencesDialog {
            add = &adw::PreferencesPage {
                add = &adw::PreferencesGroup {
                    set_title: "Views",
                    set_description: Some("Show or hide sidebar views"),

                    adw::SwitchRow {
                        set_title: "Selfies",
                        set_subtitle: "Shows a separate view for selfies taken on iOS devices. Restart Fotema to apply.",

                        #[watch]
                        set_active: model.show_selfies,

		                connect_active_notify[sender] => move |switch| {
		                    sender.input_sender().send(PreferencesInput::ShowSelfies(switch.is_active())).unwrap();
		                },
                    }
                }
            }
        }
    }


    fn init(
        parent: Self::Init,
        dialog: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {


        let settings = gio::Settings::new(APP_ID);
        let show_selfies = settings.boolean("show-selfies");

        let model = Self {
            parent,
            dialog: dialog.clone(),
            show_selfies,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PreferencesInput::Present => {
                let settings = gio::Settings::new(APP_ID);
                self.show_selfies = settings.boolean("show-selfies");
                self.dialog.present(&self.parent);
            },
            PreferencesInput::ShowSelfies(visible) => {
                let settings = gio::Settings::new(APP_ID);
                self.show_selfies = visible;

                settings.set_boolean("show-selfies", visible).expect("Update settings");

                sender.output(PreferencesOutput::Updated).expect("Sending update prefs");
            },
        }
    }
}
