// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use super::face_thumbnails::{FaceThumbnails, FaceThumbnailsInput};
/// Properties view for a photo.
///Inspired by how Loupe displays its property view.
use fotema_core::VisualId;
use fotema_core::people;
use fotema_core::FlatpakPathBuf;

use gtk::prelude::OrientableExt;

use chrono::{DateTime, Utc};
use glycin::ImageDetails;
use humansize::{DECIMAL, format_size};
use relm4::adw::prelude::*;
use relm4::gtk;
use relm4::gtk::gio;
use relm4::prelude::*;
use std::fs;
use std::sync::Arc;

use crate::app::SharedState;
use crate::fl;

use tracing::{debug, warn};

#[derive(Debug)]
pub enum ViewInfoInput {
    /// Show photo details
    Photo(VisualId, ImageDetails),

    /// Show video details
    Video(VisualId),

    /// Show file-only details.
    FileOnly(VisualId),

    OpenFolder,

    /// Refresh faces
    RefreshFaces,
}

pub struct ViewInfo {
    state: SharedState,

    path: Option<FlatpakPathBuf>,

    folder: adw::ActionRow,
    file_name: adw::ActionRow,

    // FIXME what timestamps to show for live photos that have an image an a video?
    date_time_details: adw::PreferencesGroup,
    created_at: adw::ActionRow,
    modified_at: adw::ActionRow,

    image_details: adw::PreferencesGroup,
    image_size: adw::ActionRow,
    image_format: adw::ActionRow,
    image_file_size: adw::ActionRow,

    exif_details: adw::PreferencesGroup,
    exif_originally_created_at: adw::ActionRow,
    exif_originally_modified_at: adw::ActionRow,

    video_details: adw::PreferencesGroup,
    video_dimensions: adw::ActionRow,
    video_container_format: adw::ActionRow,
    video_codec: adw::ActionRow,
    video_audio_codec: adw::ActionRow,
    video_file_size: adw::ActionRow,
    video_originally_created_at: adw::ActionRow,
    video_duration: adw::ActionRow,

    faces_row: adw::ActionRow,
    face_thumbnails: AsyncController<FaceThumbnails>,
}

#[relm4::component(pub)]
impl SimpleComponent for ViewInfo {
    type Init = (SharedState, people::Repository);
    type Input = ViewInfoInput;
    type Output = ();

    view! {
        gtk::ScrolledWindow {
            set_hscrollbar_policy: gtk::PolicyType::Never,
            set_propagate_natural_height: true,

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 12,
                set_spacing: 12,

                adw::PreferencesGroup {
                    #[local_ref]
                    folder -> adw::ActionRow {
                        set_title: &fl!("infobar-folder"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("file-cabinet-symbolic"),
                        },

                        add_suffix = &gtk::Button {
                            set_valign: gtk::Align::Center,
                            set_icon_name: "folder-open-symbolic",
                            set_tooltip_text: Some(&fl!("infobar-folder", "tooltip")),
                            add_css_class: "flat",
                            connect_clicked => ViewInfoInput::OpenFolder,
                        }
                    },

                    #[local_ref]
                    file_name -> adw::ActionRow {
                        set_title: &fl!("infobar-file-name"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("image-alt-symbolic"),
                        }
                    },
                },

                #[local_ref]
                date_time_details -> adw::PreferencesGroup {
                    #[local_ref]
                    created_at -> adw::ActionRow {
                        set_title: &fl!("infobar-file-created"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("today-symbolic"),
                        }
                    },

                    #[local_ref]
                    modified_at -> adw::ActionRow {
                        set_title: &fl!("infobar-file-modified"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("today-symbolic"),
                        }
                    },
                },

                #[local_ref]
                image_details -> adw::PreferencesGroup {
                    #[local_ref]
                    image_size -> adw::ActionRow {
                        set_title: &fl!("infobar-dimensions"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,

                        add_prefix = &gtk::Image {
                            set_icon_name: Some("ruler-corner-symbolic"),
                        }
                    },

                    #[local_ref]
                    image_format -> adw::ActionRow {
                        set_title: &fl!("infobar-file-format"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("checkerboard-symbolic"),
                        }
                    },

                    #[local_ref]
                    image_file_size -> adw::ActionRow {
                        set_title: &fl!("infobar-file-size"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("weight-symbolic"),
                        }
                    },

                    #[local_ref]
                    faces_row -> adw::ActionRow {
                        add_css_class: "property",
                        set_subtitle_selectable: false,
                        add_prefix = &gtk::Image {
                            set_valign: gtk::Align::Start,
                            set_icon_name: Some("people-symbolic"),
                        },
                        add_suffix = model.face_thumbnails.widget(),
                    },
                },

                #[local_ref]
                exif_details -> adw::PreferencesGroup {
                    #[local_ref]
                    exif_originally_created_at -> adw::ActionRow {
                        set_title: &fl!("infobar-originally-created"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("today-symbolic"),
                        }
                    },

                    #[local_ref]
                    exif_originally_modified_at -> adw::ActionRow {
                        set_title: &fl!("infobar-originally-modified"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("today-symbolic"),
                        }
                    },
                },


                #[local_ref]
                video_details -> adw::PreferencesGroup {
                    #[local_ref]
                    video_duration -> adw::ActionRow {
                        set_title: &fl!("infobar-video-duration"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("stopwatch-symbolic"),
                        }
                    },

                    #[local_ref]
                    video_dimensions -> adw::ActionRow {
                        set_title: &fl!("infobar-dimensions"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("ruler-corner-symbolic"),
                        }
                    },

                    #[local_ref]
                    video_file_size -> adw::ActionRow {
                        set_title: &fl!("infobar-file-size"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("weight-symbolic"),
                        }
                    },

                    #[local_ref]
                    video_originally_created_at -> adw::ActionRow {
                        set_title: &fl!("infobar-originally-created"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("today-symbolic"),
                        }
                    },

                    #[local_ref]
                    video_container_format -> adw::ActionRow {
                        set_title: &fl!("infobar-video-container-format"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("checkerboard-symbolic"),
                        }
                    },

                    #[local_ref]
                    video_codec -> adw::ActionRow {
                        set_title: &fl!("infobar-video-codec"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("video-reel-symbolic"),
                        }
                    },

                    #[local_ref]
                    video_audio_codec -> adw::ActionRow {
                        set_title: &fl!("infobar-audio-codec"),
                        add_css_class: "property",
                        set_subtitle_selectable: true,
                        add_prefix = &gtk::Image {
                            set_icon_name: Some("sound-wave-symbolic"),
                        }
                    },
                },
            }
        }
    }

    fn init(
        (state, people_repo): Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let folder = adw::ActionRow::new();
        let file_name = adw::ActionRow::new();

        let date_time_details = adw::PreferencesGroup::new();
        let created_at = adw::ActionRow::new();
        let modified_at = adw::ActionRow::new();

        let image_details = adw::PreferencesGroup::new();
        let image_size = adw::ActionRow::new();
        let image_format = adw::ActionRow::new();
        let image_file_size = adw::ActionRow::new();

        let exif_details = adw::PreferencesGroup::new();
        let exif_originally_created_at = adw::ActionRow::new();
        let exif_originally_modified_at = adw::ActionRow::new();

        let video_details = adw::PreferencesGroup::new();
        let video_duration = adw::ActionRow::new();
        let video_dimensions = adw::ActionRow::new();
        let video_container_format = adw::ActionRow::new();
        let video_codec = adw::ActionRow::new();
        let video_audio_codec = adw::ActionRow::new();
        let video_file_size = adw::ActionRow::new();
        let video_originally_created_at = adw::ActionRow::new();

        let faces_row = adw::ActionRow::new();
        let face_thumbnails = FaceThumbnails::builder().launch(people_repo).detach();

        let model = ViewInfo {
            state,

            folder: folder.clone(),
            file_name: file_name.clone(),
            path: None,

            date_time_details: date_time_details.clone(),
            created_at: created_at.clone(),
            modified_at: modified_at.clone(),

            image_details: image_details.clone(),
            image_size: image_size.clone(),
            image_format: image_format.clone(),
            image_file_size: image_file_size.clone(),

            exif_details: exif_details.clone(),
            exif_originally_created_at: exif_originally_created_at.clone(),
            exif_originally_modified_at: exif_originally_modified_at.clone(),

            video_details: video_details.clone(),
            video_file_size: video_file_size.clone(),
            video_originally_created_at: video_originally_created_at.clone(),
            video_duration: video_duration.clone(),
            video_container_format: video_container_format.clone(),
            video_codec: video_codec.clone(),
            video_audio_codec: video_audio_codec.clone(),
            video_dimensions: video_dimensions.clone(),

            faces_row: faces_row.clone(),
            face_thumbnails,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ViewInfoInput::OpenFolder => {
                // FIXME using self.host_path works when run in GNOME Builder, but
                // doesn't work in the Flatpak sandbox.
                let Some(ref path) = self.path.as_ref().map(|p| &p.sandbox_path) else {
                    return;
                };

                debug!("Trying to open folder for {}", path.to_string_lossy());

                let file = gtk::gio::File::for_path(path);
                let launcher = gtk::FileLauncher::new(Some(&file));
                launcher.open_containing_folder(
                    None::<&adw::ApplicationWindow>,
                    None::<&gio::Cancellable>,
                    |_| (),
                );
            }
            ViewInfoInput::FileOnly(ref visual_id) => {
                let result = {
                    let data = self.state.read();
                    data.iter().find(|&x| x.visual_id == *visual_id).cloned()
                };

                let Some(ref vis) = result else {
                    warn!("No visual item");
                    return;
                };

                self.video_details.set_visible(false);
                self.image_details.set_visible(false);
                self.exif_details.set_visible(false);

                let _ = self.update_file_details(vis.clone());
            }
            ViewInfoInput::Photo(ref visual_id, ref image_info) => {
                let result = {
                    let data = self.state.read();
                    data.iter().find(|&x| x.visual_id == *visual_id).cloned()
                };

                let Some(ref vis) = result else {
                    warn!("No visual item");
                    return;
                };

                self.video_details.set_visible(false);

                let _ = self.update_file_details(vis.clone());

                if let Some(picture_id) = vis.picture_id {
                    self.faces_row.set_visible(true);
                    let _ = self.update_photo_details(vis.clone(), image_info);
                    self.face_thumbnails
                        .emit(FaceThumbnailsInput::View(picture_id));
                } else {
                    self.faces_row.set_visible(false);
                    self.face_thumbnails.emit(FaceThumbnailsInput::Hide);
                }
            }
            ViewInfoInput::Video(ref visual_id) => {
                self.faces_row.set_visible(false);
                self.face_thumbnails.emit(FaceThumbnailsInput::Hide);

                let result = {
                    let data = self.state.read();
                    data.iter().find(|&x| x.visual_id == *visual_id).cloned()
                };

                let Some(ref vis) = result else {
                    warn!("No visual item");
                    return;
                };

                self.image_details.set_visible(false);
                self.exif_details.set_visible(false);

                let _ = self.update_file_details(vis.clone());

                if vis.video_id.is_some() {
                    let _ = self.update_video_details(vis.clone());
                }
            }
            ViewInfoInput::RefreshFaces => {
                self.face_thumbnails.emit(FaceThumbnailsInput::Refresh);
            }
        }
    }
}

/// Value row subtitle when value absent.
const FALLBACK: &str = "–";

impl ViewInfo {
    fn update_file_details(&mut self, vis: Arc<fotema_core::visual::Visual>) -> Result<(), String> {

        self.path = Some(vis.path().clone());

        Self::update_row(&self.folder, vis.folder_name());
        Self::update_row(
            &self.file_name,
            vis.host_path().file_name().map(|p| p.to_string_lossy()),
        );

        // FIXME duplicated from Scanner
        let file = fs::File::open(vis.sandbox_path()).map_err(|e| e.to_string())?;

        let metadata = file.metadata().map_err(|e| e.to_string())?;

        let fs_created_at: Option<String> = metadata
            .created()
            .map(Into::<DateTime<Utc>>::into)
            .map(|x| x.format("%Y-%m-%d %H:%M:%S %:z").to_string())
            .ok();

        let fs_modified_at: Option<String> = metadata
            .modified()
            .map(Into::<DateTime<Utc>>::into)
            .map(|x| x.format("%Y-%m-%d %H:%M:%S %:z").to_string())
            .ok();

        let has_date_time_details = [
            Self::update_row(&self.created_at, fs_created_at),
            Self::update_row(&self.modified_at, fs_modified_at),
        ]
        .into_iter()
        .any(|x| x);

        self.date_time_details.set_visible(has_date_time_details);

        Ok(())
    }

    fn update_photo_details(
        &mut self,
        vis: Arc<fotema_core::visual::Visual>,
        image_details: &ImageDetails,
    ) -> Result<(), String> {
        let Some(ref picture_path) = vis.picture_path else {
            return Err("No picture path".to_string());
        };

        // FIXME duplicated from Scanner
        let file = fs::File::open(&picture_path.sandbox_path).map_err(|e| e.to_string())?;
        let metadata = file.metadata().map_err(|e| e.to_string())?;

        let fs_file_size_bytes = metadata.len();

        let image_size = format!("{} ⨉ {}", image_details.width(), image_details.height());

        let has_image_details = [
            Self::update_row(&self.image_size, Some(image_size)),
            Self::update_row(&self.image_format, image_details.info_format_name().as_ref()),
            Self::update_row(
                &self.image_file_size,
                Some(format_size(fs_file_size_bytes, DECIMAL)),
            ),
        ]
        .into_iter()
        .any(|x| x);

        self.image_details.set_visible(has_image_details);

        if let Some(Ok(exif)) = image_details.metadata_exif().as_ref().map(|x| x.get_full()) {
            let metadata = fotema_core::photo::metadata::from_raw(exif).ok();

            let fs_created_at: Option<String> = metadata
                .clone()
                .and_then(|x| x.fs_created_at)
                .map(|x| x.format("%Y-%m-%d %H:%M:%S %:z").to_string());

            let fs_modified_at: Option<String> = metadata
                .clone()
                .and_then(|x| x.fs_modified_at)
                .map(|x| x.format("%Y-%m-%d %H:%M:%S %:z").to_string());

            let has_exif_details = [
                Self::update_row(&self.exif_originally_created_at, fs_created_at),
                Self::update_row(&self.exif_originally_modified_at, fs_modified_at),
            ]
            .into_iter()
            .any(|x| x);

            self.exif_details.set_visible(has_exif_details);
        } else {
            self.exif_details.set_visible(false);
        }

        Ok(())
    }

    fn update_video_details(
        &mut self,
        vis: Arc<fotema_core::visual::Visual>,
    ) -> Result<(), String> {
        let Some(ref video_path) = vis.video_path else {
            return Err("No video path".to_string());
        };

        // FIXME duplicated from Scanner
        let file = fs::File::open(&video_path.sandbox_path).map_err(|e| e.to_string())?;
        let fs_file_size_bytes = file.metadata().ok().map(|x| format_size(x.len(), DECIMAL));

        let metadata = fotema_core::video::metadata::from_path(&video_path.sandbox_path).ok();
        if metadata.is_none() {
            self.video_details.set_visible(false);
        }

        let metadata = metadata.expect("metadata must be present");

        let created_at: Option<String> = metadata
            .stream_created_at
            .or(metadata.fs_created_at)
            .map(|x| x.format("%Y-%m-%d %H:%M:%S %:z").to_string());

        let duration = metadata
            .duration
            .map(|ref x| fotema_core::time::format_hhmmss(x));

        let dimensions = if let (Some(width), Some(height)) = (metadata.width, metadata.height) {
            Some(format!("{} ⨉ {}", width, height))
        } else {
            None
        };

        let has_video_details = [
            Self::update_row(&self.video_originally_created_at, created_at),
            Self::update_row(&self.video_duration, duration),
            Self::update_row(&self.video_dimensions, dimensions),
            Self::update_row(&self.video_container_format, metadata.container_format),
            Self::update_row(&self.video_codec, metadata.video_codec),
            Self::update_row(&self.video_audio_codec, metadata.audio_codec),
            Self::update_row(&self.video_file_size, fs_file_size_bytes),
        ]
        .into_iter()
        .any(|x| x);

        self.video_details.set_visible(has_video_details);

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
}
