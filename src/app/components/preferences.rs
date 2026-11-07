// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use ashpd::{WindowIdentifier, desktop::file_chooser::OpenFileRequest};

use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::prelude::*;

use gtk::glib;

use tracing::{error, info};

use crate::app::{AlbumSort, App, FaceDetectionMode, Settings, SettingsState};
use crate::fl;
use crate::host_path;
use fotema_core::FlatpakPathBuf;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
enum DirectoryValidationError {
    Duplicate,
    Nested,
}

fn validate_candidate_dir(
    candidate: &FlatpakPathBuf,
    existing: &[FlatpakPathBuf],
    skip_index: Option<usize>,
) -> Option<DirectoryValidationError> {
    for (idx, dir) in existing.iter().enumerate() {
        if Some(idx) == skip_index {
            continue;
        }
        if candidate.host_path == dir.host_path || candidate.sandbox_path == dir.sandbox_path {
            return Some(DirectoryValidationError::Duplicate);
        }
        if candidate.host_path.starts_with(&dir.host_path)
            || dir.host_path.starts_with(&candidate.host_path)
            || candidate.sandbox_path.starts_with(&dir.sandbox_path)
            || dir.sandbox_path.starts_with(&candidate.sandbox_path)
        {
            return Some(DirectoryValidationError::Nested);
        }
    }
    None
}

fn is_external_library_dir(dir: &FlatpakPathBuf) -> bool {
    dir.host_path.starts_with("/mnt")
        || dir.host_path.starts_with("/media")
        || dir.host_path.starts_with("/run/media")
}

pub struct PreferencesDialog {
    parent: adw::ApplicationWindow,
    dialog: adw::PreferencesDialog,
    album_sort: adw::ComboRow,
    library_group: adw::PreferencesGroup,
    library_rows: RefCell<Vec<gtk::Widget>>,

    settings_state: SettingsState,

    // Preference values
    settings: Settings,
}

impl PreferencesDialog {
    pub fn is_face_detection_active(&self) -> bool {
        self.settings.face_detection_mode == FaceDetectionMode::On
    }

    fn show_validation_error(&self, error: DirectoryValidationError) {
        let body_text = match error {
            DirectoryValidationError::Duplicate => fl!("prefs-library-error-duplicate"),
            DirectoryValidationError::Nested => fl!("prefs-library-error-nested"),
        };
        let dialog = adw::AlertDialog::builder()
            .heading(fl!("prefs-library-error-title"))
            .body(body_text)
            .build();
        dialog.add_response("ok", &fl!("prefs-library-error-ok"));
        dialog.set_default_response(Some("ok"));
        dialog.set_close_response("ok");
        dialog.present(Some(&self.dialog));
    }

    fn update_library_rows(&self, sender: &AsyncComponentSender<Self>) {
        for row in self.library_rows.borrow_mut().drain(..) {
            self.library_group.remove(&row);
        }

        // Primary directory row (row 0)
        if let Some(primary_dir) = self.settings.library_base_dirs.first() {
            let row = adw::ActionRow::builder()
                .title(&fl!("prefs-library-section-pictures-dir", "title"))
                .subtitle(&primary_dir.host_path.to_string_lossy().to_string())
                .build();

            if is_external_library_dir(primary_dir) {
                let refresh_btn = gtk::Button::builder()
                    .valign(gtk::Align::Center)
                    .icon_name("view-refresh-symbolic")
                    .tooltip_text(&fl!("prefs-library-section-reload-pictures-dir", "tooltip"))
                    .build();

                let sender_clone = sender.clone();
                refresh_btn.connect_clicked(move |_| {
                    sender_clone.input(PreferencesInput::RefreshPicturesDir(0));
                });
                row.add_suffix(&refresh_btn);
            }

            let btn = gtk::Button::builder()
                .valign(gtk::Align::Center)
                .icon_name("folder-open-symbolic")
                .tooltip_text(&fl!("prefs-library-section-pictures-dir", "tooltip"))
                .build();

            let sender_clone = sender.clone();
            btn.connect_clicked(move |_| {
                sender_clone.input(PreferencesInput::ChangePrimaryPicturesDir);
            });
            row.add_suffix(&btn);
            self.library_group.add(&row);
            self.library_rows.borrow_mut().push(row.upcast());
        }

        // Additional directory rows (rows 1..)
        for (idx, dir) in self.settings.library_base_dirs.iter().enumerate().skip(1) {
            let folder_name = dir
                .host_path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| fl!("prefs-library-section-pictures-dir", "title"));

            let row = adw::ActionRow::builder()
                .title(&folder_name)
                .subtitle(&dir.host_path.to_string_lossy().to_string())
                .build();

            if is_external_library_dir(dir) {
                let refresh_btn = gtk::Button::builder()
                    .valign(gtk::Align::Center)
                    .icon_name("view-refresh-symbolic")
                    .tooltip_text(&fl!("prefs-library-section-reload-pictures-dir", "tooltip"))
                    .build();

                let sender_clone = sender.clone();
                refresh_btn.connect_clicked(move |_| {
                    sender_clone.input(PreferencesInput::RefreshPicturesDir(idx));
                });
                row.add_suffix(&refresh_btn);
            }

            let btn = gtk::Button::builder()
                .valign(gtk::Align::Center)
                .icon_name("user-trash-symbolic")
                .tooltip_text(&fl!("prefs-library-section-remove-pictures-dir", "tooltip"))
                .build();

            let sender_clone = sender.clone();
            btn.connect_clicked(move |_| {
                sender_clone.input(PreferencesInput::RemovePicturesDir(idx));
            });
            row.add_suffix(&btn);
            self.library_group.add(&row);
            self.library_rows.borrow_mut().push(row.upcast());
        }

        // Add Pictures Directory row
        let add_row = adw::ActionRow::builder()
            .title(&fl!("prefs-library-section-add-pictures-dir", "title"))
            .activatable(true)
            .build();

        let add_btn = gtk::Button::builder()
            .valign(gtk::Align::Center)
            .icon_name("list-add-symbolic")
            .tooltip_text(&fl!("prefs-library-section-add-pictures-dir", "tooltip"))
            .build();

        let sender_clone = sender.clone();
        add_btn.connect_clicked(move |_| {
            sender_clone.input(PreferencesInput::AddPicturesDir);
        });
        add_row.add_suffix(&add_btn);

        let sender_clone2 = sender.clone();
        add_row.connect_activated(move |_| {
            sender_clone2.input(PreferencesInput::AddPicturesDir);
        });

        self.library_group.add(&add_row);
        self.library_rows.borrow_mut().push(add_row.upcast());
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

    ChangePrimaryPicturesDir,

    AddPicturesDir,

    /// Reload an external library directory after its drive becomes available.
    RefreshPicturesDir(usize),

    RemovePicturesDir(usize),
}

// Note that some settings update through the shared state, and others through output messages.
/// Should probably make this consistent or draw a clear line between what updates when.
#[derive(Debug)]
pub enum PreferencesOutput {
    /// Start face detection right now.
    EnableFaceDetection,

    /// Process motion photos right now.
    ProcessMotionPhotos,

    /// Reload the configured library after an external drive becomes available.
    RefreshLibrary,
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

                #[local_ref]
                library_group -> adw::PreferencesGroup {},

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

        let library_group = adw::PreferencesGroup::builder()
            .title(&fl!("prefs-library-section", "title"))
            .description(&fl!("prefs-library-section", "description"))
            .build();

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
            library_group: library_group.clone(),
            library_rows: RefCell::new(Vec::new()),
        };

        let widgets = view_output!();

        model.update_library_rows(&sender);
        sender.input(PreferencesInput::SettingsChanged(model.settings.clone()));

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            PreferencesInput::Present => {
                self.settings = self.settings_state.read().clone();
                self.update_library_rows(&sender);
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
                self.update_library_rows(&sender);
            }
            PreferencesInput::UpdateShowSelfies(show_selfies) => {
                info!("Update show selfies: {}", show_selfies);
                self.settings.show_selfies = show_selfies;
                let _ = App::save_settings(&self.settings);
                *self.settings_state.write() = self.settings.clone();
            }
            PreferencesInput::UpdateProcessMotionPhotos(enable) => {
                info!("Update process motion photos: {:?}", enable);
                self.settings.process_motion_photos = enable;
                let _ = App::save_settings(&self.settings);
                *self.settings_state.write() = self.settings.clone();
                if enable {
                    let _ = sender.output(PreferencesOutput::ProcessMotionPhotos);
                }
            }
            PreferencesInput::UpdateFaceDetectionMode(mode) => {
                info!("Update face detection mode: {:?}", mode);
                self.settings.face_detection_mode = mode;
                let _ = App::save_settings(&self.settings);
                *self.settings_state.write() = self.settings.clone();
                if mode == FaceDetectionMode::On {
                    let _ = sender.output(PreferencesOutput::EnableFaceDetection);
                }
            }
            PreferencesInput::Sort(mode) => {
                info!("Update album sort: {:?}", mode);
                self.settings.album_sort = mode;
                let _ = App::save_settings(&self.settings);
                *self.settings_state.write() = self.settings.clone();
            }
            PreferencesInput::ChangePrimaryPicturesDir => {
                info!("Presenting select pictures directory file chooser for primary directory");
                if let Some(root) = gtk::Widget::root(self.parent.widget_ref()) {
                    let identifier = WindowIdentifier::from_native(&root).await;
                    let request = OpenFileRequest::default()
                        .directory(true)
                        .identifier(identifier)
                        .modal(true)
                        .multiple(false);

                    match request.send().await.and_then(|r| r.response()) {
                        Ok(files) => {
                            if let Some(uri) = files.uris().first() {
                                if let Ok(glib_uri) = glib::Uri::parse(uri.as_str(), glib::UriFlags::NONE) {
                                    let chosen_path = PathBuf::from(glib_uri.path());
                                    let flatpak_dir = host_path::host_path(&chosen_path)
                                        .await
                                        .unwrap_or(FlatpakPathBuf::build(&chosen_path, &chosen_path));

                                    if let Some(err) = validate_candidate_dir(&flatpak_dir, &self.settings.library_base_dirs, Some(0)) {
                                        self.show_validation_error(err);
                                    } else {
                                        if self.settings.library_base_dirs.is_empty() {
                                            self.settings.library_base_dirs.push(flatpak_dir);
                                        } else {
                                            self.settings.library_base_dirs[0] = flatpak_dir;
                                        }
                                        let _ = App::save_settings(&self.settings);
                                        *self.settings_state.write() = self.settings.clone();
                                        self.update_library_rows(&sender);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("Failed to open a directory: {err}");
                        }
                    }
                }
            }
            PreferencesInput::AddPicturesDir => {
                info!("Presenting select pictures directory file chooser to add directory");
                if let Some(root) = gtk::Widget::root(self.parent.widget_ref()) {
                    let identifier = WindowIdentifier::from_native(&root).await;
                    let request = OpenFileRequest::default()
                        .directory(true)
                        .identifier(identifier)
                        .modal(true)
                        .multiple(false);

                    match request.send().await.and_then(|r| r.response()) {
                        Ok(files) => {
                            if let Some(uri) = files.uris().first() {
                                if let Ok(glib_uri) = glib::Uri::parse(uri.as_str(), glib::UriFlags::NONE) {
                                    let chosen_path = PathBuf::from(glib_uri.path());
                                    let flatpak_dir = host_path::host_path(&chosen_path)
                                        .await
                                        .unwrap_or(FlatpakPathBuf::build(&chosen_path, &chosen_path));

                                    if let Some(err) = validate_candidate_dir(&flatpak_dir, &self.settings.library_base_dirs, None) {
                                        self.show_validation_error(err);
                                    } else {
                                        self.settings.library_base_dirs.push(flatpak_dir);
                                        let _ = App::save_settings(&self.settings);
                                        *self.settings_state.write() = self.settings.clone();
                                        self.update_library_rows(&sender);
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            error!("Failed to open a directory: {err}");
                        }
                    }
                }
            }
            PreferencesInput::RefreshPicturesDir(index) => {
                let Some(dir) = self
                    .settings
                    .library_base_dirs
                    .get(index)
                else {
                    return;
                };

                if dir.exists() {
                    info!("Refreshing accessible external library directory");
                    let _ = sender.output(PreferencesOutput::RefreshLibrary);
                } else {
                    let dialog = adw::AlertDialog::builder()
                        .heading(fl!("prefs-library-unavailable-title"))
                        .body(fl!("prefs-library-unavailable-body"))
                        .build();
                    dialog.add_response("ok", &fl!("prefs-library-error-ok"));
                    dialog.set_default_response(Some("ok"));
                    dialog.set_close_response("ok");
                    dialog.present(Some(&self.dialog));
                }
            }
            PreferencesInput::RemovePicturesDir(index) => {
                info!("Removing pictures directory at index {}", index);
                if index > 0 && index < self.settings.library_base_dirs.len() {
                    self.settings.library_base_dirs.remove(index);
                    let _ = App::save_settings(&self.settings);
                    *self.settings_state.write() = self.settings.clone();
                    self.update_library_rows(&sender);
                }
            }
        }
    }
}
