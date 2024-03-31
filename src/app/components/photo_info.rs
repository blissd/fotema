// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use photos_core::Scanner;
use gtk::prelude::OrientableExt;
use relm4::gtk;
use relm4::*;
use relm4::adw::prelude::PreferencesRowExt;
use relm4::adw::prelude::ActionRowExt;
use relm4::gtk::prelude::WidgetExt;


#[derive(Debug)]
pub struct PhotoInfo {
    scanner: Scanner,
}


#[relm4::component(pub)]
impl SimpleComponent for PhotoInfo {
    type Init = Scanner;
    type Input = ();
    type Output = ();

    view! {
       gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_margin_all: 10,

            adw::PreferencesGroup {
                adw::ActionRow {
                    set_title: "Test Title",
                    set_subtitle: "Subtitle",
                    add_css_class: "property",
                    set_subtitle_selectable: true,
                },

                adw::ActionRow {
                    set_title: "Another Title",
                }
            }
        }
    }

    fn init(
        scanner: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let widgets = view_output!();

        let model = PhotoInfo {
            scanner,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {}
}

