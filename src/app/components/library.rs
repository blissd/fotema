// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use fotema_core::{VisualId, YearMonth};

use relm4::adw;
use relm4::*;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::Arc;

use crate::app::ActiveView;
use crate::app::SharedState;
use crate::app::ViewName;
use crate::app::adaptive;
use crate::fl;

use super::albums::album::{Album, AlbumInput, AlbumOutput};
use super::albums::album_filter::AlbumFilter;
use super::albums::album_sort::AlbumSort;
use super::albums::months_album::{MonthsAlbum, MonthsAlbumInput, MonthsAlbumOutput};
use super::albums::years_album::{YearsAlbum, YearsAlbumInput, YearsAlbumOutput};

use crate::app::background::lazy_thumbnail_notifier::LazyThumbnailNotifier;
use crate::app::background::lazy_thumbnail_task::LazyThumbnailTaskInput;
use crate::app::background::lazy_thumbnail_tracker::LazyThumbnailTracker;

use fotema_core::thumbnailify::Thumbnailer;

#[derive(Debug)]
pub enum LibraryInput {
    Activate,

    /// Ignore an event
    Ignore,

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

    active_view: ActiveView,

    lazy_thumbnail_task_sender: Sender<LazyThumbnailTaskInput>,
}

#[relm4::component(pub)]
impl SimpleComponent for Library {
    type Init = (
        SharedState,
        ActiveView,
        Arc<adaptive::LayoutState>,
        Rc<Thumbnailer>,
        Sender<LazyThumbnailTaskInput>,
        LazyThumbnailNotifier,
    );
    type Input = LibraryInput;
    type Output = LibraryOutput;

    view! {
        adw::ViewStack {
            add_titled_with_icon[Some(ViewName::All.as_ref()), &fl!("all-album"), "today-symbolic"] = all_album.widget(),
            add_titled_with_icon[Some(ViewName::Month.as_ref()), &fl!("months-album"), "month-symbolic"] = months_album.widget(),
            add_titled_with_icon[Some(ViewName::Year.as_ref()), &fl!("years-album"), "year-symbolic"] = years_album.widget(),
            connect_visible_child_notify => LibraryInput::Activate,
        },
    }

    fn init(
        (
            state,
            active_view,
            layout_state,
            thumbnailer,
            lazy_thumbnail_task_sender,
            lazy_thumbnail_notifier,
        ): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let lazy_thumbnail_tracker = Rc::new(RefCell::new(LazyThumbnailTracker::new(
            thumbnailer.clone(),
            lazy_thumbnail_task_sender.clone(),
        )));
        let all_album = Album::builder()
            .launch((
                state.clone(),
                active_view.clone(),
                ViewName::All,
                AlbumFilter::All,
                thumbnailer.clone(),
                lazy_thumbnail_tracker,
            ))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, _) => LibraryInput::View(id),
                AlbumOutput::ScrollOffset(_) => LibraryInput::Ignore,
            });

        active_view.subscribe(all_album.sender(), |view_name| {
            AlbumInput::Activated(*view_name)
        });
        state.subscribe(all_album.sender(), |_| AlbumInput::Refresh);
        layout_state.subscribe(all_album.sender(), |layout| AlbumInput::Adapt(*layout));
        lazy_thumbnail_notifier.subscribe_optional(all_album.sender(), |notifier| {
            notifier
                .visual_id
                .clone()
                .map(|visual_id| AlbumInput::ThumbnailReady(visual_id))
        });

        let lazy_thumbnail_tracker = Rc::new(RefCell::new(LazyThumbnailTracker::new(
            thumbnailer.clone(),
            lazy_thumbnail_task_sender.clone(),
        )));
        let months_album = MonthsAlbum::builder()
            .launch((
                state.clone(),
                active_view.clone(),
                thumbnailer.clone(),
                lazy_thumbnail_tracker,
            ))
            .forward(sender.input_sender(), |msg| match msg {
                MonthsAlbumOutput::MonthSelected(ym) => LibraryInput::GoToMonth(ym),
            });

        active_view.subscribe(months_album.sender(), |view_name| {
            MonthsAlbumInput::Activated(*view_name)
        });
        state.subscribe(months_album.sender(), |_| MonthsAlbumInput::Refresh);
        layout_state.subscribe(months_album.sender(), |layout| {
            MonthsAlbumInput::Adapt(*layout)
        });
        lazy_thumbnail_notifier.subscribe_optional(months_album.sender(), |notifier| {
            notifier
                .visual_id
                .clone()
                .map(|visual_id| MonthsAlbumInput::ThumbnailReady(visual_id))
        });

        let lazy_thumbnail_tracker = Rc::new(RefCell::new(LazyThumbnailTracker::new(
            thumbnailer.clone(),
            lazy_thumbnail_task_sender.clone(),
        )));
        let years_album = YearsAlbum::builder()
            .launch((
                state.clone(),
                active_view.clone(),
                thumbnailer,
                lazy_thumbnail_tracker,
            ))
            .forward(sender.input_sender(), |msg| match msg {
                YearsAlbumOutput::YearSelected(year) => LibraryInput::GoToYear(year),
            });

        active_view.subscribe(years_album.sender(), |view_name| {
            YearsAlbumInput::Activated(*view_name)
        });
        state.subscribe(years_album.sender(), |_| YearsAlbumInput::Refresh);
        layout_state.subscribe(years_album.sender(), |layout| {
            YearsAlbumInput::Adapt(*layout)
        });
        lazy_thumbnail_notifier.subscribe_optional(years_album.sender(), |notifier| {
            notifier
                .visual_id
                .clone()
                .map(|visual_id| YearsAlbumInput::ThumbnailReady(visual_id))
        });

        let widgets = view_output!();

        let model = Library {
            stack: root,
            all_album,
            months_album,
            years_album,
            active_view,
            lazy_thumbnail_task_sender,
        };

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            LibraryInput::Activate => {
                let child_name = self
                    .stack
                    .visible_child_name()
                    .and_then(|x| ViewName::from_str(x.as_str()).ok())
                    .unwrap_or(ViewName::Nothing);

                *self.active_view.write() = child_name;
            }
            LibraryInput::Ignore => {
                // info!("Intentionally ignoring a message");
            }
            LibraryInput::GoToMonth(ym) => {
                // Display all photos view.
                self.stack.set_visible_child_name(ViewName::All.as_ref());
                self.all_album.emit(AlbumInput::Activate);
                self.all_album.emit(AlbumInput::GoToMonth(ym));
            }
            LibraryInput::GoToYear(year) => {
                // Display month photos view.
                self.stack.set_visible_child_name(ViewName::Month.as_ref());
                self.months_album.emit(MonthsAlbumInput::Activate);
                self.months_album.emit(MonthsAlbumInput::GoToYear(year));
            }
            LibraryInput::View(id) => {
                let _ = sender.output(LibraryOutput::View(id));
            }
            LibraryInput::Sort(sort) => {
                self.all_album.emit(AlbumInput::Sort(sort));
                self.months_album.emit(MonthsAlbumInput::Sort(sort));
                self.years_album.emit(YearsAlbumInput::Sort(sort));
            }
        }
    }
}
