// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;

use fotema_core::people;

use relm4::binding::*;
use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::prelude::WidgetExt;
use relm4::gtk::prelude::*;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;

use crate::adaptive;
use crate::app::ActiveView;
use crate::app::FaceDetectionMode;
use crate::app::SettingsState;
use crate::app::ViewName;
use crate::fl;

use tracing::{debug, info};

const NARROW_EDGE_LENGTH: i32 = 170;
const WIDE_EDGE_LENGTH: i32 = 200;

#[derive(Debug)]
struct PhotoGridItem {
    /// Person for avatar
    person: people::Person,

    // Length of thumbnail edge to allow for resizing when layout changes.
    edge_length: I32Binding,
}

struct Widgets {
    avatar: adw::Avatar,

    label: gtk::Label,

    // If the avatar has been bound to edge_length.
    is_bound: bool,
}
#[derive(Debug)]
pub enum PeopleAlbumInput {
    Activate,

    // Reload photos from database
    Refresh,

    Selected(u32), // Index into photo grid vector

    // Adapt to layout
    Adapt(adaptive::Layout),

    SettingsChanged,

    EnableFaceDetection,
}

#[derive(Debug)]
pub enum PeopleAlbumOutput {
    Selected(people::Person),

    EnableFaceDetection,
}

impl RelmGridItem for PhotoGridItem {
    type Root = gtk::Box;
    type Widgets = Widgets;

    fn setup(_item: &gtk::ListItem) -> (gtk::Box, Widgets) {
        relm4::view! {
           my_box = gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[name(avatar)]
                adw::Avatar {
                    set_size: NARROW_EDGE_LENGTH,
                    set_show_initials: true,
                },

                #[name(label)]
                gtk::Label {
                    add_css_class: "caption-heading",
                    set_margin_top: 4,
                    set_margin_bottom: 12,
                },
            }
        }

        let widgets = Widgets {
            avatar,
            label,
            is_bound: false,
        };

        (my_box, widgets)
    }

    fn bind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.label.set_text(&self.person.name);

        // If we repeatedly bind, then Fotema will die with the following error:
        // (fotema:2): GLib-GObject-CRITICAL **: 13:26:14.297: Too many GWeakRef registered
        // GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        // Bail out! GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        if !widgets.is_bound {
            widgets
                .avatar
                .add_write_only_binding(&self.edge_length, "size");
            widgets.is_bound = true;
        }

        widgets.avatar.set_text(Some(&self.person.name));

        if let Some(ref thumbnail_path) = self.person.thumbnail_path() {
            if thumbnail_path.exists() {
                let img = gdk::Texture::from_filename(thumbnail_path).ok();
                widgets.avatar.set_custom_image(img.as_ref());
            }
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.avatar.set_custom_image(None::<&gdk::Paintable>);
        widgets.avatar.set_text(None);
    }
}

pub struct PeopleAlbum {
    repo: people::Repository,
    active_view: ActiveView,
    settings_state: SettingsState,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
    avatars: gtk::ScrolledWindow,
    status: adw::StatusPage,
    edge_length: I32Binding,
}

#[relm4::component(pub)]
impl SimpleComponent for PeopleAlbum {
    type Init = (people::Repository, ActiveView, SettingsState);
    type Input = PeopleAlbumInput;
    type Output = PeopleAlbumOutput;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            #[local_ref]
            avatars -> gtk::ScrolledWindow {
                set_vexpand: true,

                #[local_ref]
                pictures_box -> gtk::GridView {
                    set_orientation: gtk::Orientation::Vertical,
                    set_single_click_activate: true,

                    connect_activate[sender] => move |_, idx| {
                        sender.input(PeopleAlbumInput::Selected(idx))
                    }
                }
            },

            #[local_ref]
            status -> adw::StatusPage {
                set_valign: gtk::Align::Start,
                set_vexpand: true,

                set_visible: false,
                set_icon_name: Some("sentiment-very-satisfied-symbolic"),

                #[wrap(Some)]
                set_child = &adw::Clamp {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_maximum_size: 400,

                    #[wrap(Some)]
                    set_child = &gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::Button {
                            set_label: &fl!("people-page-status-off", "enable"),
                            //add_css_class: "suggested-action",
                            add_css_class: "pill",
                            connect_clicked => PeopleAlbumInput::EnableFaceDetection,
                        },
                    }
                }
            },
        },
    }

    fn init(
        (repo, active_view, settings_state): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        settings_state.subscribe(sender.input_sender(), |_| PeopleAlbumInput::SettingsChanged);

        let photo_grid = TypedGridView::new();

        let status = adw::StatusPage::new();

        let avatars = gtk::ScrolledWindow::builder().build();

        let model = PeopleAlbum {
            repo,
            active_view,
            settings_state,
            photo_grid,
            avatars: avatars.clone(),
            status: status.clone(),
            edge_length: I32Binding::new(NARROW_EDGE_LENGTH),
        };

        let pictures_box = &model.photo_grid.view;

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            PeopleAlbumInput::Activate => {
                info!("Activating people view");
                *self.active_view.write() = ViewName::People;
                self.refresh();
            }
            PeopleAlbumInput::Selected(index) => {
                debug!("Person selected index: {}", index);
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let item = item.borrow();
                    debug!("Person selected item: {}", item.person.person_id);
                    //let picture_ids = self.repo.find_pictures_for_person(item.person.person_id).unwrap_or(vec![]);

                    let _ = sender.output(PeopleAlbumOutput::Selected(item.person.clone()));
                }
            }
            PeopleAlbumInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            }
            PeopleAlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            }
            PeopleAlbumInput::Refresh => {
                self.refresh();
            }
            PeopleAlbumInput::SettingsChanged => {
                self.refresh();
            }
            PeopleAlbumInput::EnableFaceDetection => {
                let mut settings = self.settings_state.read().clone();
                settings.face_detection_mode = FaceDetectionMode::On;
                *self.settings_state.write() = settings;
                self.refresh();
                let _ = sender.output(PeopleAlbumOutput::EnableFaceDetection);
            }
        }
    }
}

impl PeopleAlbum {
    fn refresh(&mut self) {
        if self.settings_state.read().face_detection_mode == FaceDetectionMode::Off {
            self.avatars.set_visible(false);
            self.status.set_visible(true);
            self.status
                .set_title(&fl!("people-page-status-off", "title"));
            self.status
                .set_description(Some(&fl!("people-page-status-off", "description")));

            if let Some(child) = self.status.child() {
                child.set_visible(true);
            }
            return;
        }

        let mut people = self.repo.all_people().unwrap_or_default();
        people.sort_by_key(|p| p.name.clone());

        self.photo_grid.clear();

        let mut items = vec![];
        for person in people {
            let item = PhotoGridItem {
                person,
                edge_length: self.edge_length.clone(),
            };

            items.push(item);
        }

        self.status.set_visible(items.is_empty());
        self.avatars.set_visible(!items.is_empty());

        if items.is_empty() {
            if let Some(child) = self.status.child() {
                child.set_visible(false);
            }
            self.status
                .set_title(&fl!("people-page-status-no-people", "title"));
            self.status
                .set_description(Some(&fl!("people-page-status-no-people", "description")));
        }

        self.photo_grid.extend_from_iter(items);
    }
}
