// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::{VisualId, YearMonth};

use relm4::*;
use relm4::adw;
use std::str::FromStr;
use std::sync::Arc;
use strum::EnumString;
use strum::IntoStaticStr;

use crate::app::adaptive;
use crate::app::SharedState;
use crate::app::ActiveView;
use crate::app::ViewName;
use crate::fl;

use super::albums::album::{Album, AlbumInput, AlbumOutput};
use super::albums::album_filter::AlbumFilter;
use super::albums::album_sort::AlbumSort;
use super::albums::months_album::{MonthsAlbum, MonthsAlbumInput, MonthsAlbumOutput};
use super::albums::years_album::{YearsAlbum, YearsAlbumInput, YearsAlbumOutput};

use tracing::error;

#[derive(Debug)]
pub enum LibraryInput {
    /// Ignore an event
    Ignore,

    // Library view is activated
    Activate,

    // Reload photos from database
    //Refresh,

    // Scroll to first photo in month
    GoToMonth(YearMonth),

    // Scroll to first photo in year
    GoToYear(i32),

    View(VisualId),

    Sort(AlbumSort),
}

#[derive(Debug)]
pub enum LibraryOutput {
    View(VisualId),
}


pub struct Library {
    stack: adw::ViewStack,

    all_album: Controller<Album>,

    months_album: Controller<MonthsAlbum>,

    years_album: Controller<YearsAlbum>,
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
    type Init = (SharedState, ActiveView, Arc<adaptive::LayoutState>);
    type Input = LibraryInput;
    type Output = LibraryOutput;

    view! {
        adw::ViewStack {
            add_titled_with_icon[Some(LibraryViewName::All.into()), &fl!("all-album"), "playlist-infinite-symbolic"] = all_album.widget(),
            add_titled_with_icon[Some(LibraryViewName::Month.into()), &fl!("months-album"), "month-symbolic"] = months_album.widget(),
            add_titled_with_icon[Some(LibraryViewName::Year.into()), &fl!("years-album"), "year-symbolic"] = years_album.widget(),
            connect_visible_child_notify => LibraryInput::Activate,
        },
    }

    fn init(
        (state, active_view, layout_state): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {

        let all_album = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::All, AlbumFilter::All))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, _) => LibraryInput::View(id),
                AlbumOutput::ScrollOffset(_) => LibraryInput::Ignore,
            });

        state.subscribe(all_album.sender(), |_| AlbumInput::Refresh);
        layout_state.subscribe(all_album.sender(), |layout| AlbumInput::Adapt(*layout));

        let months_album = MonthsAlbum::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                MonthsAlbumOutput::MonthSelected(ym) => LibraryInput::GoToMonth(ym),
            },
        );

        state.subscribe(months_album.sender(), |_| MonthsAlbumInput::Refresh);
        layout_state.subscribe(months_album.sender(), |layout| MonthsAlbumInput::Adapt(*layout));

        let years_album = YearsAlbum::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(sender.input_sender(),|msg| match msg {
                YearsAlbumOutput::YearSelected(year) => LibraryInput::GoToYear(year),
            },
        );

        state.subscribe(years_album.sender(), |_| YearsAlbumInput::Refresh);
        layout_state.subscribe(years_album.sender(), |layout| YearsAlbumInput::Adapt(*layout));

        let widgets = view_output!();

        let model = Library {
            stack: root,
            all_album,
            months_album,
            years_album,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LibraryInput::Ignore => {
                // info!("Intentionally ignoring a message");
            },
            LibraryInput::Activate => {
                let child_name = self.stack.visible_child_name()
                    .and_then(|x| LibraryViewName::from_str(x.as_str()).ok())
                    .unwrap_or(LibraryViewName::Nothing);

                match child_name {
                    LibraryViewName::All => self.all_album.emit(AlbumInput::Activate),
                    LibraryViewName::Month => self.months_album.emit(MonthsAlbumInput::Activate),
                    LibraryViewName::Year => self.years_album.emit(YearsAlbumInput::Activate),
                    LibraryViewName::Nothing => error!("Nothing activated for library view :-/"),
                }
            },
            LibraryInput::GoToMonth(ym) => {
                // Display all photos view.
                self.stack.set_visible_child_name(LibraryViewName::All.into());
                self.all_album.emit(AlbumInput::Activate);
                self.all_album.emit(AlbumInput::GoToMonth(ym));
            },
            LibraryInput::GoToYear(year) => {
                // Display month photos view.
                self.stack.set_visible_child_name(LibraryViewName::Month.into());
                self.months_album.emit(MonthsAlbumInput::Activate);
                self.months_album.emit(MonthsAlbumInput::GoToYear(year));
            },
            LibraryInput::View(id) => {
                let _ = sender.output(LibraryOutput::View(id));
            },
            LibraryInput::Sort(sort) => {
                self.all_album.emit(AlbumInput::Sort(sort));
                self.months_album.emit(MonthsAlbumInput::Sort(sort));
            },
        }
    }
}
