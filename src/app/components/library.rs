// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::{VisualId, YearMonth};


use relm4::*;
use relm4::adw;


use crate::app::SharedState;
use super::album::{Album, AlbumFilter, AlbumInput, AlbumOutput};
use super::month_photos::{MonthPhotos, MonthPhotosInput, MonthPhotosOutput};
use super::year_photos::{YearPhotos, YearPhotosInput, YearPhotosOutput};

#[derive(Debug)]
pub enum LibraryInput {
   // Activate,

    // Reload photos from database
    //Refresh,

    // Scroll to first photo in month
    GoToMonth(YearMonth),

    // Scroll to first photo in year
    GoToYear(i32),

    ViewPhoto(VisualId),
}

#[derive(Debug)]
pub enum LibraryOutput {
    ViewPhoto(VisualId),
}


pub struct Library {
    state: SharedState,

    stack: adw::ViewStack,

    all_photos: Controller<Album>,

    month_photos: Controller<MonthPhotos>,

    year_photos: Controller<YearPhotos>,
}

#[relm4::component(pub)]
impl SimpleComponent for Library {
    type Init = SharedState;
    type Input = LibraryInput;
    type Output = LibraryOutput;

    view! {
        adw::ViewStack {
            add_titled_with_icon[Some("all"), "All", "playlist-infinite-symbolic"] = all_photos.widget(),
            add_titled_with_icon[Some("month"), "Month", "month-symbolic"] = month_photos.widget(),
            add_titled_with_icon[Some("year"), "Year", "year-symbolic"] = year_photos.widget(),
        },
    }

    fn init(
        state: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let all_photos = Album::builder()
            .launch((state.clone(), AlbumFilter::All))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id) => LibraryInput::ViewPhoto(id),
            });

        state.subscribe(all_photos.sender(), |_| AlbumInput::Refresh);

        let month_photos = MonthPhotos::builder()
            .launch(state.clone()).forward(
            sender.input_sender(),
            |msg| match msg {
                MonthPhotosOutput::MonthSelected(ym) => LibraryInput::GoToMonth(ym),
            },
        );

        state.subscribe(month_photos.sender(), |_| MonthPhotosInput::Refresh);

        let year_photos = YearPhotos::builder()
            .launch(state.clone()).forward(
            sender.input_sender(),
            |msg| match msg {
                YearPhotosOutput::YearSelected(year) => LibraryInput::GoToYear(year),
            },
        );

        state.subscribe(year_photos.sender(), |_| YearPhotosInput::Refresh);

       // let stack = adw::ViewStack::new();



        let widgets = view_output!();

        let model = Library {
            state,
            stack: root,
            all_photos,
            month_photos,
            year_photos,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LibraryInput::GoToMonth(ym) => {
                // Display all photos view.
                self.stack.set_visible_child_name("all");
                self.all_photos.emit(AlbumInput::GoToMonth(ym));
            },
            LibraryInput::GoToYear(year) => {
                // Display month photos view.
                self.stack.set_visible_child_name("month");
                self.month_photos.emit(MonthPhotosInput::GoToYear(year));
            },
            LibraryInput::ViewPhoto(id) => {
                let _ = sender.output(LibraryOutput::ViewPhoto(id));
            },
        }
    }
}
