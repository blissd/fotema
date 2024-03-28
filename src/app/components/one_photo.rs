// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::prelude::{ OrientableExt};
use photos_core::repo::PictureId;
use relm4::gtk;
use relm4::gtk::prelude::WidgetExt;
use relm4::*;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub enum OnePhotoInput {
    ViewPhoto(PictureId),
}

#[derive(Debug)]
pub struct OnePhoto {
    controller: Rc<RefCell<photos_core::Controller>>,
    picture: gtk::Picture,
}

#[relm4::component(pub)]
impl SimpleComponent for OnePhoto {
    type Init = Rc<RefCell<photos_core::Controller>>;
    type Input = OnePhotoInput;
    type Output = ();

    view! {
        adw::ToolbarView {
            add_top_bar = &adw::HeaderBar,

            #[wrap(Some)]
            set_content = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,

                #[local_ref]
                picture -> gtk::Picture {
                    set_can_shrink: true,
                    set_valign: gtk::Align::Center,
                }
            }
        }
    }

    fn init(
        controller: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let picture = gtk::Picture::new();

        let widgets = view_output!();

        let model = OnePhoto {
            controller,
            picture,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
     match msg {
            OnePhotoInput::ViewPhoto(picture_id) => {
                println!("Showing photo for {}", picture_id);
                let result = self.controller.borrow_mut().get(picture_id);
                if let Ok(Some(pic)) = result {
                    self.picture.set_filename(Some(pic.path));
                    //self.picture_navigation_view.push_by_tag("picture");
                } else {
                    println!("Failed loading {}: {:?}", picture_id, result);
                }
            }
        }
    }
}
