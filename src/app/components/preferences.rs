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
use crate::app::AlbumSort;

pub struct PreferencesDialog {
    parent: adw::ApplicationWindow,
    dialog: adw::PreferencesDialog,
    album_sort: adw::ComboRow,

    settings_state: SettingsState,

    // Preference values
    settings: Settings,
}

impl PreferencesDialog {
    pub fn is_face_detection_active(&self) -> bool {
        self.settings.face_detection_mode == FaceDetectionMode::On
    }
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

    Sort(AlbumSort),
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
                    face_detection_mode_row -> adw::SwitchRow {
                        set_title: &fl!("prefs-views-faces"),
                        set_subtitle: &fl!("prefs-views-faces", "subtitle"),

                        #[watch]
                        set_active: model.is_face_detection_active(),

                        connect_active_notify[sender] => move |switch| {
                            let mode = if switch.is_active() {
                                FaceDetectionMode::On
                            } else {
                                FaceDetectionMode::Off
                            };
                            let _ = sender.input_sender().send(PreferencesInput::UpdateFaceDetectionMode(mode));
                        },
                    },

                    #[local_ref]
                    album_sort_row -> adw::ComboRow {
                        set_title: "Album sort",
                        //set_subtitle: &fl!("prefs-views-faces", "subtitle"),

                        connect_selected_item_notify[sender] => move |row| {
                            let mode = AlbumSort::from_repr(row.selected()).unwrap_or_default();
                            let _ = sender.input_sender().send(PreferencesInput::Sort(mode));
                        }
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

        let face_detection_mode_row = adw::SwitchRow::builder()
            .active(settings_state.read().face_detection_mode == FaceDetectionMode::On)
            .build();

        let album_sort_row = adw::ComboRow::new();
        let list = gtk::StringList::new(&[
            "Ascending",
            "Descending",
        ]);
        album_sort_row.set_model(Some(&list));

        let model = Self {
            settings_state: settings_state.clone(),
            parent,
            dialog: dialog.clone(),
            settings: settings_state.read().clone(),
            album_sort: album_sort_row.clone(),
        };

        let widgets = view_output!();

        sender.input(PreferencesInput::SettingsChanged(model.settings.clone()));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PreferencesInput::Present => {
                self.settings = self.settings_state.read().clone();
                self.dialog.present(Some(&self.parent));
            },
            PreferencesInput::SettingsChanged(settings) => {
                info!("Received update from settings shared state");
                self.settings = settings;

                let index = match self.settings.album_sort {
                    AlbumSort::Ascending => 0,
                    AlbumSort::Descending => 1,
                };

                self.album_sort.set_selected(index);
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
            PreferencesInput::Sort(mode) => {
                info!("Update album sort: {:?}", mode);
                self.settings.album_sort = mode;
                *self.settings_state.write() = self.settings.clone();
            },
        }
    }
}
