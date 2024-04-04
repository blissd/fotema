// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

/// Properties view for a photo.
/// Deeply inspired by how Loupe displays its property view.

use photos_core::Scanner;
use gtk::prelude::OrientableExt;
use relm4::gtk;
use relm4::*;
use relm4::adw::prelude::*;
use std::path::PathBuf;
use humansize::{format_size, DECIMAL};

#[derive(Debug)]
pub enum PhotoInfoInput {
    ShowInfo(PathBuf),
}

#[derive(Debug)]
pub struct PhotoInfo {
    scanner: Scanner,

    folder: adw::ActionRow,

    date_time_details: adw::PreferencesGroup,
    created_at: adw::ActionRow,
    modified_at: adw::ActionRow,

    image_details: adw::PreferencesGroup,
    image_size: adw::ActionRow,
    image_format: adw::ActionRow,
    file_size: adw::ActionRow,

    exif_details: adw::PreferencesGroup,
    originally_created_at: adw::ActionRow,
    originally_modified_at: adw::ActionRow,
}


#[relm4::component(pub)]
impl SimpleComponent for PhotoInfo {
    type Init = Scanner;
    type Input = PhotoInfoInput;
    type Output = ();

    view! {
       gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 12,
            set_spacing: 12,

            adw::PreferencesGroup {
                #[local_ref]
                folder -> adw::ActionRow {
                    set_title: "Folder",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },
            },

            #[local_ref]
            date_time_details -> adw::PreferencesGroup {
                #[local_ref]
                created_at -> adw::ActionRow {
                    set_title: "File Created",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },

                #[local_ref]
                modified_at -> adw::ActionRow {
                    set_title: "File Modified",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },
            },

            #[local_ref]
            image_details -> adw::PreferencesGroup {
                #[local_ref]
                image_size -> adw::ActionRow {
                    set_title: "Image Size",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },

                #[local_ref]
                image_format -> adw::ActionRow {
                    set_title: "Image Format",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },

                #[local_ref]
                file_size -> adw::ActionRow {
                    set_title: "File Size",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },
            },

            #[local_ref]
            exif_details -> adw::PreferencesGroup {
                #[local_ref]
                originally_created_at -> adw::ActionRow {
                    set_title: "Originally Created",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },

                #[local_ref]
                originally_modified_at -> adw::ActionRow {
                    set_title: "Originally Modified",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },
            }
        }
    }

    fn init(
        scanner: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let folder = adw::ActionRow::new();

        let date_time_details = adw::PreferencesGroup::new();
        let created_at = adw::ActionRow::new();
        let modified_at = adw::ActionRow::new();

        let image_details = adw::PreferencesGroup::new();
        let image_size = adw::ActionRow::new();
        let image_format = adw::ActionRow::new();
        let file_size = adw::ActionRow::new();

        let exif_details = adw::PreferencesGroup::new();
        let originally_created_at = adw::ActionRow::new();
        let originally_modified_at = adw::ActionRow::new();

        let model = PhotoInfo {
            scanner,
            folder: folder.clone(),

            date_time_details: date_time_details.clone(),
            created_at: created_at.clone(),
            modified_at: modified_at.clone(),

            image_details: image_details.clone(),
            image_size: image_size.clone(),
            image_format: image_format.clone(),
            file_size: file_size.clone(),

            exif_details: exif_details.clone(),
            originally_created_at: originally_created_at.clone(),
            originally_modified_at: originally_modified_at.clone(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PhotoInfoInput::ShowInfo(ref path) => {
                println!("Received {:?}", msg);
                self.update_pic_info(path);
            }
        }
    }
}

/// Value row subtitle when value absent.
const FALLBACK: &str = "–";

impl PhotoInfo {

    fn update_pic_info(&mut self, path: &PathBuf) {
        let result = self.scanner.scan_one(path);
        let Ok(pic) = result else {
            println!("Failed scanning picture: {:?}", result);
            return;
        };

        Self::update_row(&self.folder, Self::folder_name(path));

        let has_date_time_details = [
            Self::update_row(&self.created_at, pic.fs.as_ref().and_then(|fs| fs.created_at.map(|x| x.to_string()))),
            Self::update_row(&self.modified_at, pic.fs.as_ref().and_then(|fs| fs.modified_at.map(|x| x.to_string()))),
        ]
        .into_iter()
        .any(|x| x);

        self.date_time_details.set_visible(has_date_time_details);

        let has_image_details = [
            Self::update_row(&self.image_size, pic.image_size.map(|x| x.to_string())),
            Self::update_row(&self.image_format, pic.image_format.map(|x| x.to_string())),
            Self::update_row(&self.file_size, pic.fs.and_then(|fs| fs.file_size_bytes.map(|x| format_size(x, DECIMAL)))),
        ]
        .into_iter()
        .any(|x| x);

        self.image_details.set_visible(has_image_details);

        let has_exif_details = [
            Self::update_row(&self.originally_created_at, pic.exif.as_ref().and_then(|exif| exif.created_at.map(|x| x.to_string()))),
            Self::update_row(&self.originally_modified_at, pic.exif.as_ref().and_then(|exif| exif.modified_at.map(|x| x.to_string()))),
        ]
        .into_iter()
        .any(|x| x);

        self.exif_details.set_visible(has_exif_details);
    }

    /// Borrowed from Loupe.
    /// Updates a row to be visible if it has a value to display, and returns
    /// visibility status.
    fn update_row(row: &adw::ActionRow, value: Option<impl AsRef<str>>) -> bool {
        if let Some(value) = value {
            row.set_subtitle(value.as_ref());
            row.set_visible(true);
            true
        } else {
            row.set_subtitle(FALLBACK);
            row.set_visible(false);
            false
        }
    }

    fn folder_name(path: &PathBuf) -> Option<String> {
        path.parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy())
            .map(|n| n.to_string())
    }

}
