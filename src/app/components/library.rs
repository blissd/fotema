// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::{VisualId, YearMonth};

use relm4::*;
use relm4::adw;
use std::str::FromStr;
use strum::EnumString;
use strum::IntoStaticStr;

use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;

use super::album::{Album, AlbumFilter, AlbumInput, AlbumOutput};
use super::month_photos::{MonthPhotos, MonthPhotosInput, MonthPhotosOutput};
use super::year_photos::{YearPhotos, YearPhotosInput, YearPhotosOutput};

#[derive(Debug)]
pub enum LibraryInput {
    // Library view is activated
    Activate,

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
    stack: adw::ViewStack,

    all_photos: Controller<Album>,

    month_photos: Controller<MonthPhotos>,

    year_photos: Controller<YearPhotos>,
}

#[derive(Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
enum LibraryViewName {
    Nothing, // no active child when first created
    All,
    Month,
    Year,
}

#[relm4::component(pub)]
impl SimpleComponent for Library {
    type Init = (SharedState, ActiveView);
    type Input = LibraryInput;
    type Output = LibraryOutput;

    view! {
        adw::ViewStack {
            add_titled_with_icon[Some(LibraryViewName::All.into()), "All", "playlist-infinite-symbolic"] = all_photos.widget(),
            add_titled_with_icon[Some(LibraryViewName::Month.into()), "Month", "month-symbolic"] = month_photos.widget(),
            add_titled_with_icon[Some(LibraryViewName::Year.into()), "Year", "year-symbolic"] = year_photos.widget(),
            connect_visible_child_notify => LibraryInput::Activate,
        },
    }

    fn init(
        (state, active_view): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let all_photos = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::All, AlbumFilter::All))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id) => LibraryInput::ViewPhoto(id),
            });

        state.subscribe(all_photos.sender(), |_| AlbumInput::Refresh);

        let month_photos = MonthPhotos::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                MonthPhotosOutput::MonthSelected(ym) => LibraryInput::GoToMonth(ym),
            },
        );

        state.subscribe(month_photos.sender(), |_| MonthPhotosInput::Refresh);

        let year_photos = YearPhotos::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(sender.input_sender(),|msg| match msg {
                YearPhotosOutput::YearSelected(year) => LibraryInput::GoToYear(year),
            },
        );

        state.subscribe(year_photos.sender(), |_| YearPhotosInput::Refresh);

        let widgets = view_output!();

        let model = Library {
            stack: root,
            all_photos,
            month_photos,
            year_photos,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LibraryInput::Activate => {
                let child_name = self.stack.visible_child_name()
                    .and_then(|x| LibraryViewName::from_str(x.as_str()).ok())
                    .unwrap_or(LibraryViewName::Nothing);

                match child_name {
                    LibraryViewName::All => self.all_photos.emit(AlbumInput::Activate),
                    LibraryViewName::Month => self.month_photos.emit(MonthPhotosInput::Activate),
                    LibraryViewName::Year => self.year_photos.emit(YearPhotosInput::Activate),
                    LibraryViewName::Nothing => println!("Nothing activated for library view :-/"),
                }
            }
            LibraryInput::GoToMonth(ym) => {
                // Display all photos view.
                self.stack.set_visible_child_name(LibraryViewName::All.into());
                self.all_photos.emit(AlbumInput::Activate);
                self.all_photos.emit(AlbumInput::GoToMonth(ym));
            },
            LibraryInput::GoToYear(year) => {
                // Display month photos view.
                self.stack.set_visible_child_name(LibraryViewName::Month.into());
                self.month_photos.emit(MonthPhotosInput::Activate);
                self.month_photos.emit(MonthPhotosInput::GoToYear(year));
            },
            LibraryInput::ViewPhoto(id) => {
                let _ = sender.output(LibraryOutput::ViewPhoto(id));
            },
        }
    }
}
