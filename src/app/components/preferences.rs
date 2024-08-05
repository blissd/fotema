// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{adw, ComponentParts, ComponentSender, SimpleComponent};
use relm4::adw::prelude::AdwDialogExt;
use relm4::adw::prelude::PreferencesDialogExt;
use relm4::adw::prelude::PreferencesPageExt;
use relm4::adw::prelude::PreferencesGroupExt;
use relm4::adw::prelude::ActionRowExt;
use relm4::adw::prelude::PreferencesRowExt;

use tracing::info;

use crate::fl;
use crate::app::{Settings, SettingsState};

pub struct PreferencesDialog {
    parent: adw::ApplicationWindow,
    dialog: adw::PreferencesDialog,
    settings_state: SettingsState,

    // Preference values
    settings: Settings,
}

#[derive(Debug)]
pub enum PreferencesInput {
    /// Show the preferences dialog.
    Present,

    /// Changed settings received.
    SettingsChanged(Settings),

    /// Send updated settings
    UpdateShowSelfies(bool),
}

#[relm4::component(pub)]
impl SimpleComponent for PreferencesDialog {
    type Init = (SettingsState, adw::ApplicationWindow);
    type Input = PreferencesInput;
    type Output = ();

    view!{
        adw::PreferencesDialog {
            set_title: &fl!("prefs-title"),
            add = &adw::PreferencesPage {
                add = &adw::PreferencesGroup {
                    set_title: &fl!("prefs-views-section"),
                    set_description: Some(&fl!("prefs-views-section", "description")),

                    adw::SwitchRow {
                        set_title: &fl!("prefs-views-selfies"),
                        set_subtitle: &fl!("prefs-views-selfies", "subtitle"),

                        #[watch]
                        set_active: model.settings.show_selfies,

		                connect_active_notify[sender] => move |switch| {
		                    let _ = sender.input_sender().send(PreferencesInput::UpdateShowSelfies(switch.is_active()));
		                },
                    }
                }
            }
        }
    }


    fn init(
        (settings_state, parent): Self::Init,
        dialog: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        settings_state.subscribe(sender.input_sender(), |settings| PreferencesInput::SettingsChanged(settings.clone()));

        let model = Self {
            settings_state: settings_state.clone(),
            parent,
            dialog: dialog.clone(),
            settings: settings_state.read().clone(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PreferencesInput::Present => {
                self.settings = self.settings_state.read().clone();
                self.dialog.present(&self.parent);
            },
            PreferencesInput::SettingsChanged(settings) => {
                info!("Received update from settings sahred state");
                self.settings = settings;
            },
            PreferencesInput::UpdateShowSelfies(show_selfies) => {
                info!("Writing update to settings shared state");
                self.settings.show_selfies = show_selfies;
                *self.settings_state.write() = self.settings.clone();
            },
        }
    }
}
