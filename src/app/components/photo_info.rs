// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

/// Properties view for a photo.
/// Deeply inspired by how Loupe displays its property view.

use fotema_core::photo;
use fotema_core::Library;
use fotema_core::VisualId;
use gtk::prelude::OrientableExt;

use relm4::gtk;
use relm4::*;
use relm4::adw::prelude::*;
use std::path::PathBuf;
use humansize::{format_size, DECIMAL};
use glycin::{ImageInfo, ImageInfoDetails};
use std::fs;
use chrono::prelude::*;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveTime, Utc};


#[derive(Debug)]
pub enum PhotoInfoInput {
    ShowInfo(VisualId, ImageInfo),
}

pub struct PhotoInfo {
    photo_scan: photo::Scanner,
    library: Library,

    folder: adw::ActionRow,

    // FIXME what timestamps to show for live photos that have an image an a video?
    date_time_details: adw::PreferencesGroup,
    created_at: adw::ActionRow,
    modified_at: adw::ActionRow,

    image_details: adw::PreferencesGroup,
    image_size: adw::ActionRow,
    image_format: adw::ActionRow,
    image_file_size: adw::ActionRow,
    image_originally_created_at: adw::ActionRow,
    image_originally_modified_at: adw::ActionRow,

    video_details: adw::PreferencesGroup,
    video_format: adw::ActionRow,
    video_file_size: adw::ActionRow,
    video_originally_created_at: adw::ActionRow,
    video_originally_modified_at: adw::ActionRow,
    video_duration: adw::ActionRow,
}


#[relm4::component(pub)]
impl SimpleComponent for PhotoInfo {
    type Init = (Library, photo::Scanner);
    type Input = PhotoInfoInput;
    type Output = ();

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
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
                    image_file_size -> adw::ActionRow {
                        set_title: "File Size",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },

                    #[local_ref]
                    image_originally_created_at -> adw::ActionRow {
                        set_title: "Originally Created",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },

                    #[local_ref]
                    image_originally_modified_at -> adw::ActionRow {
                        set_title: "Originally Modified",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },
                },


                #[local_ref]
                video_details -> adw::PreferencesGroup {
                    #[local_ref]
                    video_duration -> adw::ActionRow {
                        set_title: "Duration",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },

                    #[local_ref]
                    video_format -> adw::ActionRow {
                        set_title: "Video Format",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },

                    #[local_ref]
                    video_file_size -> adw::ActionRow {
                        set_title: "File Size",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },

                    #[local_ref]
                    video_originally_created_at -> adw::ActionRow {
                        set_title: "Originally Created",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },

                    #[local_ref]
                    video_originally_modified_at -> adw::ActionRow {
                        set_title: "Originally Modified",
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                    },
                },
            }
        }
    }

    fn init(
        (library, photo_scan): Self::Init,
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
        let image_file_size = adw::ActionRow::new();
        let image_originally_created_at = adw::ActionRow::new();
        let image_originally_modified_at = adw::ActionRow::new();

        let video_details = adw::PreferencesGroup::new();
        let video_duration = adw::ActionRow::new();
        let video_format = adw::ActionRow::new();
        let video_file_size = adw::ActionRow::new();
        let video_originally_created_at = adw::ActionRow::new();
        let video_originally_modified_at = adw::ActionRow::new();

        let model = PhotoInfo {
            library,
            photo_scan,
            folder: folder.clone(),

            date_time_details: date_time_details.clone(),
            created_at: created_at.clone(),
            modified_at: modified_at.clone(),

            image_details: image_details.clone(),
            image_size: image_size.clone(),
            image_format: image_format.clone(),
            image_file_size: image_file_size.clone(),
            image_originally_created_at: image_originally_created_at.clone(),
            image_originally_modified_at: image_originally_modified_at.clone(),

            video_details: video_details.clone(),
            video_duration: video_duration.clone(),
            video_format: video_format.clone(),
            video_file_size: video_file_size.clone(),
            video_originally_created_at: video_originally_created_at.clone(),
            video_originally_modified_at: video_originally_modified_at.clone(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            PhotoInfoInput::ShowInfo(visual_id, ref image_info) => {
                println!("Received {:?}", msg);
                self.update_pic_info(visual_id, image_info);
            }
        }
    }
}

/// Value row subtitle when value absent.
const FALLBACK: &str = "–";

impl PhotoInfo {

    fn update_pic_info(&mut self, visual_id: VisualId, image_info: &ImageInfo) -> Result<(), String> {
        let result = self.library.get(visual_id);
        let Some(vis) = result else {
            return Err("No visual item".to_string());
        };

        let Some(ref picture_path) = vis.picture_path else {
            return Err("No picture path".to_string());
        };

        Self::update_row(&self.folder, Self::folder_name(&vis.parent_path));

        // FIXME duplicated from Scanner
        let file = fs::File::open(picture_path).map_err(|e| e.to_string())?;

        let metadata = file.metadata().map_err(|e| e.to_string())?;

        let fs_created_at: Option<String> = metadata
            .created()
            .map(|x| Into::<DateTime<Utc>>::into(x))
            .map(|x| x.to_string())
            .map_err(|e| e.to_string())
            .ok();


        let fs_modified_at: Option<String> = metadata
            .modified()
            .map(|x| Into::<DateTime<Utc>>::into(x))
            .map(|x| x.to_string())
            .map_err(|e| e.to_string())
            .ok();

        let fs_file_size_bytes = metadata.len();

        let has_date_time_details = [
            Self::update_row(&self.created_at, fs_created_at),
            Self::update_row(&self.modified_at, fs_modified_at),
        ]
        .into_iter()
        .any(|x| x);

        self.date_time_details.set_visible(has_date_time_details);

        let image_size = format!("{} x {}", image_info.width, image_info.height);

        let has_image_details = [
            Self::update_row(&self.image_size, Some(image_size)),
            Self::update_row(&self.image_format, image_info.details.format_name.as_ref()),
            Self::update_row(&self.image_file_size, Some(format_size(fs_file_size_bytes, DECIMAL))),
        ]
        .into_iter()
        .any(|x| x);

        self.image_details.set_visible(has_image_details);

/*
        let has_exif_details = [
            Self::update_row(&self.originally_created_at, pic.exif.as_ref().and_then(|exif| exif.created_at.map(|x| x.to_string()))),
            Self::update_row(&self.originally_modified_at, pic.exif.as_ref().and_then(|exif| exif.modified_at.map(|x| x.to_string()))),
        ]
        .into_iter()
        .any(|x| x);

        self.exif_details.set_visible(has_exif_details);
        */

        Ok(())
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
