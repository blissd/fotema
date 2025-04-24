// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use ashpd::{WindowIdentifier, desktop::file_chooser::OpenFileRequest};

use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::prelude::*;

use tracing::{error, info};

use crate::app::AlbumSort;
use crate::app::FaceDetectionMode;
use crate::app::{Settings, SettingsState};
use crate::fl;
use crate::host_path;

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

    pub fn picture_base_dir_host_path(&self) -> String {
        self.settings
            .pictures_base_dir_host_path
            .to_string_lossy()
            .to_string()
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

    UpdateProcessMotionPhotos(bool),

    Sort(AlbumSort),

    ChoosePicturesDir,
}

// Note that some settings update through the shared state, and others through output messages.
/// Should probably make this consistent or draw a clear line between what updates when.
#[derive(Debug)]
pub enum PreferencesOutput {

    /// Start face detection right now.
    EnableFaceDetection,

    /// Process motion photos right now.
    ProcessMotionPhotos,
}




#[relm4::component(pub async)]
impl SimpleAsyncComponent for PreferencesDialog {
    type Init = (SettingsState, adw::ApplicationWindow);
    type Input = PreferencesInput;
    type Output = PreferencesOutput;

    view! {
        adw::PreferencesDialog {
            set_title: &fl!("prefs-title"),
            add = &adw::PreferencesPage {

                add = &adw::PreferencesGroup {
                    set_title: &fl!("prefs-library-section", "title"),
                    set_description: Some(&fl!("prefs-library-section", "description")),

                    adw::ActionRow {
                        set_title: &fl!("prefs-library-section-pictures-dir", "title"),

                        #[watch]
                        set_subtitle: &model.picture_base_dir_host_path(),

                        add_suffix = &gtk::Button {
                            set_valign: gtk::Align::Center,
                            set_icon_name: "folder-open-symbolic",
                            set_tooltip_text: Some(&fl!("prefs-library-section-pictures-dir", "tooltip")),
                            connect_clicked => PreferencesInput::ChoosePicturesDir,
                        }
                    }
                },

                add = &adw::PreferencesGroup {
                    set_title: &fl!("prefs-albums-section"),
                    set_description: Some(&fl!("prefs-albums-section", "description")),

                    adw::SwitchRow {
                        set_title: &fl!("prefs-albums-selfies"),
                        set_subtitle: &fl!("prefs-albums-selfies", "subtitle"),

                        #[watch]
                        set_active: model.settings.show_selfies,

                        connect_active_notify[sender] => move |switch| {
                            let _ = sender.input_sender().send(PreferencesInput::UpdateShowSelfies(switch.is_active()));
                        },
                    },

                    #[local_ref]
                    album_sort_row -> adw::ComboRow {
                        set_title: &fl!("prefs-albums-chronological-sort"),
                        set_subtitle: &fl!("prefs-albums-chronological-sort", "subtitle"),

                        connect_selected_item_notify[sender] => move |row| {
                            let mode = AlbumSort::from_repr(row.selected()).unwrap_or_default();
                            let _ = sender.input_sender().send(PreferencesInput::Sort(mode));
                        }
                    }
                },
                add = &adw::PreferencesGroup {
                    set_title: &fl!("prefs-processing-section"),
                    set_description: Some(&fl!("prefs-processing-section", "description")),


                    #[local_ref]
                    face_detection_mode_row -> adw::SwitchRow {
                        set_title: &fl!("prefs-processing-face-detection"),
                        set_subtitle: &fl!("prefs-processing-face-detection", "subtitle"),

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

                    adw::SwitchRow {
                        set_title: &fl!("prefs-processing-motion-photos"),
                        set_subtitle: &fl!("prefs-processing-motion-photos", "subtitle"),

                        #[watch]
                        set_active: model.settings.process_motion_photos,

                        connect_active_notify[sender] => move |switch| {
                            let _ = sender.input_sender().send(PreferencesInput::UpdateProcessMotionPhotos(switch.is_active()));
                        },
                    },

                },
            }
        }
    }

    async fn init(
        (settings_state, parent): Self::Init,
        dialog: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        settings_state.subscribe(sender.input_sender(), |settings| {
            PreferencesInput::SettingsChanged(settings.clone())
        });

        let face_detection_mode_row = adw::SwitchRow::builder()
            .active(settings_state.read().face_detection_mode == FaceDetectionMode::On)
            .build();

        let album_sort_row = adw::ComboRow::new();
        let list = gtk::StringList::new(&[
            &fl!("prefs-albums-chronological-sort", "ascending"),
            &fl!("prefs-albums-chronological-sort", "descending"),
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

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            PreferencesInput::Present => {
                self.settings = self.settings_state.read().clone();
                self.dialog.present(Some(&self.parent));
            }
            PreferencesInput::SettingsChanged(settings) => {
                info!("Received update from settings shared state");
                self.settings = settings;

                let index = match self.settings.album_sort {
                    AlbumSort::Ascending => 0,
                    AlbumSort::Descending => 1,
                };

                self.album_sort.set_selected(index);
            }
            PreferencesInput::UpdateShowSelfies(show_selfies) => {
                info!("Update show selfies: {}", show_selfies);
                self.settings.show_selfies = show_selfies;
                *self.settings_state.write() = self.settings.clone();
            }
            PreferencesInput::UpdateProcessMotionPhotos(enable) => {
                info!("Update process motion photos: {:?}", enable);
                self.settings.process_motion_photos = enable;
                *self.settings_state.write() = self.settings.clone();
                if enable {
                    let _ = sender.output(PreferencesOutput::ProcessMotionPhotos);
                }
            }
            PreferencesInput::UpdateFaceDetectionMode(mode) => {
                info!("Update face detection mode: {:?}", mode);
                self.settings.face_detection_mode = mode;
                *self.settings_state.write() = self.settings.clone();
                if mode == FaceDetectionMode::On {
                    let _ = sender.output(PreferencesOutput::EnableFaceDetection);
                }
            }
            PreferencesInput::Sort(mode) => {
                info!("Update album sort: {:?}", mode);
                self.settings.album_sort = mode;
                *self.settings_state.write() = self.settings.clone();
            }
            PreferencesInput::ChoosePicturesDir => {
                info!("Presenting select pictures directory file chooser");
                if let Some(root) = gtk::Widget::root(self.parent.widget_ref()) {
                    let identifier = WindowIdentifier::from_native(&root).await;
                    let request = OpenFileRequest::default()
                        .directory(true)
                        .identifier(identifier)
                        .modal(true) // can't be modal without identifier.
                        .multiple(false);

                    match request.send().await.and_then(|r| r.response()) {
                        Ok(files) => {
                            info!("Open: {:?}", files);
                            if let Some(first) = files.uris().first() {
                                info!("User has chosen picture library at: {:?}", first.path());
                                if let Some(pictures_base_dir) =
                                    files.uris().first().and_then(|uri| uri.to_file_path().ok())
                                {
                                    info!(
                                        "User has chosen picture library at: {:?}",
                                        pictures_base_dir
                                    );
                                    if self.settings.pictures_base_dir != pictures_base_dir {
                                        info!(
                                            "New pictures base director is: {:?}",
                                            pictures_base_dir
                                        );
                                        self.settings.pictures_base_dir = pictures_base_dir.clone();
                                        self.settings.pictures_base_dir_host_path = host_path::host_path(&pictures_base_dir)
                                            .await.unwrap_or(pictures_base_dir);
                                        *self.settings_state.write() = self.settings.clone();
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("Failed to open a file: {err}");
                        }
                    }
                }
            }
        }
    }
}

