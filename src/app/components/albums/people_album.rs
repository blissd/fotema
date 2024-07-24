// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::OrientableExt;

use fotema_core::people::{self, PersonId};
use fotema_core::PictureId;

use relm4::gtk;
use relm4::gtk::gdk;
use relm4::gtk::prelude::WidgetExt;
use relm4::typed_view::grid::{RelmGridItem, TypedGridView};
use relm4::*;
use relm4::binding::*;

use crate::adaptive;
use crate::app::ActiveView;
use crate::app::ViewName;

use tracing::{event, Level, info};

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
    //Refresh,

    Selected(u32), // Index into photo grid vector

    // Adapt to layout
    Adapt(adaptive::Layout),
}

#[derive(Debug)]
pub enum PeopleAlbumOutput {
    Selected(PersonId, Vec<PictureId>),
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
        widgets
            .label
            .set_text(&self.person.name);

        // If we repeatedly bind, then Fotema will die with the following error:
        // (fotema:2): GLib-GObject-CRITICAL **: 13:26:14.297: Too many GWeakRef registered
        // GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        // Bail out! GLib-GObject:ERROR:../gobject/gbinding.c:805:g_binding_constructed: assertion failed: (source != NULL)
        if !widgets.is_bound {
            widgets.avatar.add_write_only_binding(&self.edge_length, "size");
            widgets.is_bound = true;
        }

        widgets.avatar.set_text(Some(&self.person.name));

        if self.person.thumbnail_path.exists() {
            let img = gdk::Texture::from_filename(&self.person.thumbnail_path).ok();
            widgets.avatar.set_custom_image(img.as_ref());
        }
    }

    fn unbind(&mut self, widgets: &mut Self::Widgets, _root: &mut Self::Root) {
        widgets.avatar.set_custom_image(None::<&gdk::Paintable>);
    }
}

pub struct PeopleAlbum {
    repo: people::Repository,
    active_view: ActiveView,
    photo_grid: TypedGridView<PhotoGridItem, gtk::SingleSelection>,
    edge_length: I32Binding,
}

#[relm4::component(pub)]
impl SimpleComponent for PeopleAlbum {
    type Init = (people::Repository, ActiveView);
    type Input = PeopleAlbumInput;
    type Output = PeopleAlbumOutput;

    view! {
        gtk::ScrolledWindow {
            set_vexpand: true,

            #[local_ref]
            pictures_box -> gtk::GridView {
                set_orientation: gtk::Orientation::Vertical,
                set_single_click_activate: true,

                connect_activate[sender] => move |_, idx| {
                    sender.input(PeopleAlbumInput::Selected(idx))
                }
            }
        }
    }

    fn init(
        (repo, active_view): Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let photo_grid = TypedGridView::new();

        let model = PeopleAlbum {
            repo,
            active_view,
            photo_grid,
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
            },
            PeopleAlbumInput::Selected(index) => {
                event!(Level::DEBUG, "Person selected index: {}", index);
                if let Some(item) = self.photo_grid.get_visible(index) {
                    let item = item.borrow();
                    event!(Level::DEBUG, "Person selected item: {}", item.person.person_id);
                    let picture_ids = self.repo.find_pictures_for_person(item.person.person_id).unwrap_or(vec![]);

                    let _ = sender.output(PeopleAlbumOutput::Selected(item.person.person_id, picture_ids));
                }
            },
            PeopleAlbumInput::Adapt(adaptive::Layout::Narrow) => {
                self.edge_length.set_value(NARROW_EDGE_LENGTH);
            },
            PeopleAlbumInput::Adapt(adaptive::Layout::Wide) => {
                self.edge_length.set_value(WIDE_EDGE_LENGTH);
            },
        }
    }
}

impl PeopleAlbum {
    fn refresh(&mut self) {
        let mut people = self.repo.all_people().unwrap_or(vec![]);
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

        self.photo_grid.extend_from_iter(items.into_iter());

        // NOTE view is not sorted by a timestamp, so don't scroll to end.
    }
}
