// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{adw, ComponentParts, ComponentSender, SimpleComponent};
use relm4::adw::prelude::*;
use relm4::gtk;

use tracing::info;

use crate::fl;
use crate::app::{Settings, SettingsState};
use crate::app::FaceDetectionMode;

pub struct PreferencesDialog {
    parent: adw::ApplicationWindow,
    face_detection_mode_row: adw::ComboRow,
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

    UpdateFaceDetectionMode(FaceDetectionMode),
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
                    },

                    #[local_ref]
                    face_detection_mode_row -> adw::ComboRow {
                        set_title: &fl!("prefs-views-faces"),
                        set_subtitle: &fl!("prefs-views-faces", "subtitle"),
                        connect_selected_item_notify[sender] => move |row| {
                            let mode = FaceDetectionMode::from_repr(row.selected()).unwrap_or_default();
                            let _ = sender.input_sender().send(PreferencesInput::UpdateFaceDetectionMode(mode));
                        },
                    }
                },
            }
        }
    }


    fn init(
        (settings_state, parent): Self::Init,
        dialog: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        settings_state.subscribe(sender.input_sender(), |settings| PreferencesInput::SettingsChanged(settings.clone()));

        let face_detection_mode_row = adw::ComboRow::new();
        let list = gtk::StringList::new(&[
            &fl!("prefs-views-faces", "off"),
            &fl!("prefs-views-faces", "enable-mobile"),
            &fl!("prefs-views-faces", "enable-desktop"),
        ]);
        face_detection_mode_row.set_model(Some(&list));

        let model = Self {
            settings_state: settings_state.clone(),
            face_detection_mode_row: face_detection_mode_row.clone(),
            parent,
            dialog: dialog.clone(),
            settings: settings_state.read().clone(),
        };

        let widgets = view_output!();

        sender.input(PreferencesInput::SettingsChanged(model.settings.clone()));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PreferencesInput::Present => {
                self.settings = self.settings_state.read().clone();
                self.dialog.present(&self.parent);
            },
            PreferencesInput::SettingsChanged(settings) => {
                info!("Received update from settings shared state");
                self.settings = settings;
                let index = match self.settings.face_detection_mode {
                    FaceDetectionMode::Off => 0,
                    FaceDetectionMode::Mobile => 1,
                    FaceDetectionMode::Desktop => 2,
                };
                self.face_detection_mode_row.set_selected(index);
            },
            PreferencesInput::UpdateShowSelfies(show_selfies) => {
                info!("Update show selfies: {}", show_selfies);
                self.settings.show_selfies = show_selfies;
                *self.settings_state.write() = self.settings.clone();
            },
            PreferencesInput::UpdateFaceDetectionMode(mode) => {
                info!("Update face detection mode: {:?}", mode);
                self.settings.face_detection_mode = mode;
                *self.settings_state.write() = self.settings.clone();
            },
        }
    }
}
