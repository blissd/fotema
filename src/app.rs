// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{
    actions::{RelmAction, RelmActionGroup},
    adw,
    adw::prelude::{AdwApplicationWindowExt, NavigationPageExt},
    component::{AsyncComponent, AsyncComponentController},
    gtk,
    gtk::{
        gio, glib,
        prelude::{
            ApplicationExt, ButtonExt, GtkWindowExt, OrientableExt,
            SettingsExt, WidgetExt,
        },
    },
    main_application,
    prelude::AsyncController,
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent, WorkerController,
    shared_state::Reducer,
};

use crate::config::{APP_ID, PROFILE};
use crate::adaptive;
use crate::fl;

use fotema_core::database;
use fotema_core::video;
use fotema_core::VisualId;
use fotema_core::PictureId;
use fotema_core::people;

use h3o::CellIndex;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::str::FromStr;

use anyhow::*;

use strum::{AsRefStr, EnumString, IntoStaticStr, FromRepr};

use tracing::{event, Level, error, info};

mod components;

use self::components::{
    about::AboutDialog,
    albums:: {
        album::{Album, AlbumInput, AlbumOutput},
        album_filter::AlbumFilter,
        folders_album::{FoldersAlbum, FoldersAlbumInput, FoldersAlbumOutput},
        people_album::{PeopleAlbum, PeopleAlbumInput, PeopleAlbumOutput},
        person_album::{PersonAlbum, PersonAlbumInput, PersonAlbumOutput},
        places_album::{PlacesAlbum, PlacesAlbumInput, PlacesAlbumOutput},
    },
    library::{Library, LibraryInput, LibraryOutput},
    viewer::view_nav::{ViewNav, ViewNavInput, ViewNavOutput},
    preferences::{PreferencesDialog, PreferencesInput},
};

mod background;

use self::background::{
    bootstrap::{Bootstrap, BootstrapInput, BootstrapOutput, TaskName, MediaType},
    video_transcode::{VideoTranscode, VideoTranscodeInput},
};

use self::components::progress_monitor::ProgressMonitor;
use self::components::progress_panel::ProgressPanel;

/// Name of a view that can be displayed
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, EnumString, IntoStaticStr)]
pub enum ViewName {
    #[default]
    Nothing, // no view
    Library, // parent of all, month, and year views.
    All,
    Month,
    Year,
    Videos,
    Animated,
    Folders,
    Folder,
    People,
    Person,
    Places,
    Selfies,
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, EnumString, AsRefStr, FromRepr)]
#[repr(u32)]
pub enum FaceDetectionMode {
    /// Disable face detection and person recognition.
    #[default]
    Off,

    // Enable
    On,
}

/// Settings the user can change in the preferences dialog.
/// Should not include any non-preference dialog settings like window size or maximization state.
#[derive(Clone, Debug, Default)]
pub struct Settings {
    /// Is selfies view enabled?
    pub show_selfies: bool,

    /// Enable or disable face detection.
    pub face_detection_mode: FaceDetectionMode,
}

/// Active settings
type SettingsState = Arc<relm4::SharedState<Settings>>;

/// Currently visible view
/// This allows a view to know if it is visible or not and to lazily load
/// images into the photo grids. Without lazy loading Fotema will take too long to
/// update its views and GNOME will tell the user "Fotema is not responding" and offer
/// to kill the app :-(
type ActiveView = Arc<relm4::SharedState<ViewName>>;

// Visual items to be shared between various views.
// State is loaded by the `load_library` background task.
type SharedState = Arc<relm4::SharedState<Vec<Arc<fotema_core::Visual>>>>;

pub(super) struct App {
    adaptive_layout: Arc<adaptive::LayoutState>,

    about_dialog: Controller<AboutDialog>,
    preferences_dialog: Controller<PreferencesDialog>,

    bootstrap: WorkerController<Bootstrap>,
    video_transcode: WorkerController<VideoTranscode>,

    library: Controller<Library>,

    view_nav: AsyncController<ViewNav>,

    show_selfies: bool,
    selfies_page: Controller<Album>,
    videos_page: Controller<Album>,
    motion_page: Controller<Album>,

    /// Album with photos overlayed onto a map
    people_page: Controller<PeopleAlbum>,

    // Album for individual person.
    person_album: Controller<PersonAlbum>,

    /// Album with photos overlayed onto a map
    places_page: Controller<PlacesAlbum>,

    // Grid of folders of photos
    folders_album: Controller<FoldersAlbum>,

    // Folder album currently being viewed
    folder_album: Controller<Album>,

    // Main navigation. Parent of library stack.
    main_navigation: adw::OverlaySplitView,

    // Stack containing Library, Selfies, Folders, etc.
    main_stack: gtk::Stack,

    // Switch between library views and single image view.
    picture_navigation_view: adw::NavigationView,

    // Window header bar
    header_bar: adw::HeaderBar,

    // Activity indicator. Only shown when progress bar is hidden.
    spinner: gtk::Spinner,

    bootstrap_progress: Controller<ProgressPanel>,
    transcode_progress: Controller<ProgressPanel>,

    // Message banner
    banner: adw::Banner,
}

#[derive(Debug)]
pub(super) enum AppMsg {
    Activate(i32),

    Quit,

    /// Ignore event
    Ignore,

    // Toggle visibility of sidebar
    ToggleSidebar,

    // A sidebar item has been clicked
    SwitchView,

    // Show item.
    View(VisualId, AlbumFilter),

    // Shown item is dismissed.
    ViewHidden,

    ViewFolder(PathBuf),

    ViewGeographicArea(CellIndex),

    ViewPerson(people::Person),

    PersonDeleted,

    PersonRenamed,

    // A background task has started.
    TaskStarted(TaskName),

    // All background bootstrap tasks have completed
    BootstrapCompleted,

    TranscodeAll,

    ScanPictureForFaces(PictureId),
    ScanPicturesForFaces,

    // Adapt to layout change
    Adapt(adaptive::Layout),

    /// Settings updated
    SettingsChanged(Settings),
}

relm4::new_action_group!(pub(super) WindowActionGroup, "win");
relm4::new_stateless_action!(PreferencesAction, WindowActionGroup, "preferences");
relm4::new_stateless_action!(AboutAction, WindowActionGroup, "about");

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type Widgets = AppWidgets;

    menu! {
        primary_menu: {
            section! {
                &fl!("primary-menu-preferences") => PreferencesAction,
                &fl!("primary-menu-about") => AboutAction,
            }
        }
    }

    view! {
        #[root]
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_visible: true,

            // See https://linuxphoneapps.org/docs/resources/developer-information/#hardware-specs-to-consider
            set_width_request: 360,
            set_height_request: 294,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

            add_css_class?: if PROFILE == "Devel" {
                    Some("devel")
                } else {
                    None
                },

            add_breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
                adw::BreakpointConditionLengthType::MaxWidth,
                500.0,
                adw::LengthUnit::Sp,
            )) {
                add_setter: (&header_bar, "show-title", Some(&false.into())),
                add_setter: (&switcher_bar, "reveal", Some(&true.into())),
                //add_setter: (&main_navigation, "collapsed", &true.into()),
                //add_setter: (&main_navigation, "show-sidebar", &false.into()),
                add_setter: (&spinner, "visible", Some(&true.into())),

                connect_apply => AppMsg::Adapt(adaptive::Layout::Narrow),
                connect_unapply => AppMsg::Adapt(adaptive::Layout::Wide),
            },

            // Top-level navigation view containing:
            // 1. Navigation view containing stack of pages.
            // 2. Page for displaying a single photo.
            #[local_ref]
            picture_navigation_view -> adw::NavigationView {
                set_pop_on_escape: true,
                connect_popped[sender] => move |_,_| sender.input(AppMsg::ViewHidden),

                // Page for showing main navigation. Such as "Library", "Selfies", etc.
                adw::NavigationPage {

                    #[local_ref]
                    main_navigation -> adw::OverlaySplitView {

                        set_max_sidebar_width: 200.0,

                        #[wrap(Some)]
                        set_sidebar = &adw::NavigationPage {
                            adw::ToolbarView {
                                add_top_bar = &adw::HeaderBar {
                                    pack_end = &gtk::MenuButton {
                                        set_icon_name: "open-menu-symbolic",
                                        set_menu_model: Some(&primary_menu),
                                    }
                                },
                                #[wrap(Some)]
                                set_content = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    gtk::StackSidebar {
                                        set_stack: &main_stack,
                                        set_vexpand: true,
                                    },

                                    model.bootstrap_progress.widget(),
                                    model.transcode_progress.widget(),
                                }
                            }
                        },

                        #[wrap(Some)]
                        set_content = &adw::NavigationPage {
                            adw::ToolbarView {
                                #[local_ref]
                                add_top_bar = &header_bar -> adw::HeaderBar {
                                    set_hexpand: true,
                                    pack_start = &gtk::Button {
                                        set_icon_name: "dock-left-symbolic",
                                        connect_clicked => AppMsg::ToggleSidebar,
                                    },

                                    #[local_ref]
                                    pack_end = &spinner -> gtk::Spinner,
                                },

                                // NOTE I would like this to be an adw::ViewStack
                                // so that I could use a adw::ViewSwitcher in the sidebar
                                // that would show icons.
                                // However, adw::ViewSwitch can't display vertically.
                                #[wrap(Some)]
                                set_content = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    #[local_ref]
                                    banner -> adw::Banner {
                                        // Only show when generating thumbnails
                                        set_button_label: None,
                                    },

                                    #[local_ref]
                                    main_stack -> gtk::Stack {
                                        connect_visible_child_notify => AppMsg::SwitchView,

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.library.widget(),

                                            #[name(switcher_bar)]
                                            adw::ViewSwitcherBar {
                                                set_stack: Some(model.library.widget()),
                                            },
                                        } -> {
                                            set_title: &fl!("library-page"),
                                            set_name: ViewName::Library.into(),

                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "image-alt-symbolic",
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.videos_page.widget(),
                                        } -> {
                                            set_title: &fl!("videos-album"),
                                            set_name: ViewName::Videos.into(),
                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "video-reel-symbolic",
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.motion_page.widget(),
                                        } -> {
                                            set_title: &fl!("animated-album"),
                                            set_name: ViewName::Animated.into(),
                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "sonar-symbolic",
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.people_page.widget(),
                                        } -> {
                                            set_title: &fl!("people-page"),
                                            set_name: ViewName::People.into(),
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.places_page.widget(),
                                        } -> {
                                            set_title: &fl!("places-page"),
                                            set_name: ViewName::Places.into(),
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.selfies_page.widget(),
                                        } -> {
                                            set_visible: model.show_selfies,
                                            set_title: &fl!("selfies-album"),
                                            set_name: ViewName::Selfies.into(),
                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "sentiment-very-satisfied-symbolic",
                                        },

                                        add_child = &adw::NavigationView {
                                            set_pop_on_escape: true,

                                            adw::NavigationPage {
                                                //set_tag: Some("folders"),
                                                //set_title: "Folder",
                                                model.folders_album.widget(),
                                            },
                                        } -> {
                                            set_title: &fl!("folders-album"),
                                            set_name: ViewName::Folders.into(),
                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "folder-symbolic",
                                        },
                                    },
                                },
                            },
                        },
                    },
                },

                adw::NavigationPage {
                    set_tag: Some("album"),
                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {
                            #[wrap(Some)]
                            set_title_widget = &gtk::Label {
                                set_label: &fl!("folder-album"),
                                add_css_class: "title",
                            }
                        },

                        #[wrap(Some)]
                        set_content = model.folder_album.widget(),
                    }
                },

                adw::NavigationPage {
                    set_tag: Some("person_album"),
                    model.person_album.widget(),
                },

                // Page for showing a single photo.
                adw::NavigationPage {
                    set_tag: Some("picture"),
                    model.view_nav.widget(),
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let data_dir = glib::user_data_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&data_dir);

        let cache_dir = glib::user_cache_dir().join(APP_ID);
        let _ = std::fs::create_dir_all(&cache_dir);

        let pic_base_dir = glib::user_special_dir(glib::enums::UserDirectory::Pictures)
            .expect("Expect XDG_PICTURES_DIR");

        info!("XDG_PICTURES_DIR is {:?}", pic_base_dir);

        let db_path = data_dir.join("pictures.sqlite");

        let con = database::setup(&db_path).expect("Must be able to open database");
        let con = Arc::new(Mutex::new(con));

        let video_repo = {
            video::Repository::open(&pic_base_dir, &cache_dir, con.clone()).unwrap()
        };

        let people_repo = people::Repository::open(
            &pic_base_dir,
            &data_dir,
            con.clone(),
        ).unwrap();

        let state = SharedState::new(relm4::SharedState::new());
        let active_view = ActiveView::new(relm4::SharedState::new());
        let adaptive_layout = Arc::new(adaptive::LayoutState::new());

        let settings_state = SettingsState::new(relm4::SharedState::new());
        match App::load_settings() {
            std::result::Result::Ok(settings) => {
                info!("Loaded settings: {:?}", settings);
                *settings_state.write() = settings;
            },
            Err(e) => error!("Failed loading settings: {}", e),
        }

        settings_state.subscribe(sender.input_sender(), |settings| AppMsg::SettingsChanged(settings.clone()));

        let bootstrap_progress_monitor: Reducer<ProgressMonitor> = Reducer::new();
        let bootstrap_progress_monitor = Arc::new(bootstrap_progress_monitor);

        let bootstrap_progress = self::components::progress_panel::ProgressPanel::builder()
            .launch(bootstrap_progress_monitor.clone())
            .detach();

        let transcode_progress_monitor: Reducer<ProgressMonitor> = Reducer::new();
        let transcode_progress_monitor = Arc::new(transcode_progress_monitor);

        let transcode_progress = self::components::progress_panel::ProgressPanel::builder()
            .launch(transcode_progress_monitor.clone())
            .detach();

        let bootstrap = Bootstrap::builder()
            .detach_worker((con.clone(), state.clone(), settings_state.clone(), bootstrap_progress_monitor))
            .forward(sender.input_sender(), |msg| match msg {
                BootstrapOutput::TaskStarted(msg) => AppMsg::TaskStarted(msg),
                BootstrapOutput::Completed => AppMsg::BootstrapCompleted,
            });

        let library = Library::builder()
            .launch((state.clone(), active_view.clone(), adaptive_layout.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                LibraryOutput::View(id) => AppMsg::View(id, AlbumFilter::All),
            });

        let transcoder = video::Transcoder::new(&cache_dir);

        let video_transcode = VideoTranscode::builder()
            .detach_worker((state.clone(), video_repo, transcoder.clone(), transcode_progress_monitor.clone()))
            .detach();

        let view_nav = ViewNav::builder()
            .launch((state.clone(), transcode_progress_monitor.clone(), adaptive_layout.clone(), people_repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ViewNavOutput::TranscodeAll => AppMsg::TranscodeAll,
                ViewNavOutput::ScanForFaces(picture_id) => AppMsg::ScanPictureForFaces(picture_id),
            });

        let selfies_page = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Selfies, AlbumFilter::Selfies))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
                AlbumOutput::ScrollOffset(_) => AppMsg::Ignore,
            });

        state.subscribe(selfies_page.sender(), |_| AlbumInput::Refresh);
        adaptive_layout.subscribe(selfies_page.sender(), |layout| AlbumInput::Adapt(*layout));

        let show_selfies = AppWidgets::show_selfies();

        let motion_page = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Animated, AlbumFilter::Motion))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
                AlbumOutput::ScrollOffset(_) => AppMsg::Ignore,
            });

        state.subscribe(motion_page.sender(), |_| AlbumInput::Refresh);
        adaptive_layout.subscribe(motion_page.sender(), |layout| AlbumInput::Adapt(*layout));

        let videos_page = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Videos, AlbumFilter::Videos))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
                AlbumOutput::ScrollOffset(_) => AppMsg::Ignore,
            });

        state.subscribe(videos_page.sender(), |_| AlbumInput::Refresh);
        adaptive_layout.subscribe(videos_page.sender(), |layout| AlbumInput::Adapt(*layout));

        let people_page = PeopleAlbum::builder()
            .launch((people_repo.clone(), active_view.clone(), settings_state.clone()))
            .forward(
            sender.input_sender(),
            |msg| match msg {
                PeopleAlbumOutput::Selected(person) => AppMsg::ViewPerson(person),
                PeopleAlbumOutput::EnableFaceDetection => AppMsg::ScanPicturesForFaces,
            },
        );

        adaptive_layout.subscribe(people_page.sender(), |layout| PeopleAlbumInput::Adapt(*layout));

        let person_album = PersonAlbum::builder()
            .launch((state.clone(), people_repo.clone(), active_view.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PersonAlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
                PersonAlbumOutput::Deleted => AppMsg::PersonDeleted,
                PersonAlbumOutput::Renamed => AppMsg::PersonRenamed,
            });

        state.subscribe(person_album.sender(), |_| PersonAlbumInput::Refresh);
        adaptive_layout.subscribe(person_album.sender(), |layout| PersonAlbumInput::Adapt(*layout));

        let places_page = PlacesAlbum::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                PlacesAlbumOutput::View(visual_id) => AppMsg::View(visual_id.clone(), AlbumFilter::One(visual_id)),
                PlacesAlbumOutput::GeographicArea(cell_index) => AppMsg::ViewGeographicArea(cell_index),
            });

        state.subscribe(places_page.sender(), |_| PlacesAlbumInput::Refresh);
        adaptive_layout.subscribe(places_page.sender(), |layout| PlacesAlbumInput::Adapt(*layout));

        let folders_album = FoldersAlbum::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(
            sender.input_sender(),
            |msg| match msg {
                FoldersAlbumOutput::FolderSelected(path) => AppMsg::ViewFolder(path),
            },
        );

        state.subscribe(folders_album.sender(), |_| FoldersAlbumInput::Refresh);
        adaptive_layout.subscribe(folders_album.sender(), |layout| FoldersAlbumInput::Adapt(*layout));

        let folder_album = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Folder, AlbumFilter::None))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
                AlbumOutput::ScrollOffset(_) => AppMsg::Ignore,
            });

        state.subscribe(folder_album.sender(), |_| AlbumInput::Refresh);
        adaptive_layout.subscribe(folder_album.sender(), |layout| AlbumInput::Adapt(*layout));

        let about_dialog = AboutDialog::builder().launch(root.clone()).detach();

        let preferences_dialog = PreferencesDialog::builder()
            .launch((settings_state.clone(), root.clone()))
            .detach();

        let picture_navigation_view = adw::NavigationView::builder().build();

        let main_navigation = adw::OverlaySplitView::builder().build();

        let main_stack = gtk::Stack::new();

        let header_bar = adw::HeaderBar::new();

        let spinner = gtk::Spinner::builder().visible(false).build();

        let banner = adw::Banner::new("-");

        let model = Self {
            adaptive_layout,
            bootstrap,
            video_transcode,

            about_dialog,
            preferences_dialog,

            library,

            view_nav,
            motion_page,
            videos_page,
            people_page,
            person_album,
            places_page,
            selfies_page,
            show_selfies,
            folders_album,
            folder_album,

            main_navigation: main_navigation.clone(),
            main_stack: main_stack.clone(),

            picture_navigation_view: picture_navigation_view.clone(),
            header_bar: header_bar.clone(),
            spinner: spinner.clone(),

            bootstrap_progress,
            transcode_progress,

            banner: banner.clone(),
        };

        let widgets = view_output!();

        let mut actions = RelmActionGroup::<WindowActionGroup>::new();

        let about_action = {
            let sender = model.about_dialog.sender().clone();
            RelmAction::<AboutAction>::new_stateless(move |_| {
                sender.send(()).unwrap();
            })
        };

        let preferences_action = {
            let sender = model.preferences_dialog.sender().clone();
            RelmAction::<PreferencesAction>::new_stateless(move |_| {
                sender.send(PreferencesInput::Present).unwrap();
            })
        };

        actions.add_action(about_action);
        actions.add_action(preferences_action);

        actions.register_for_widget(&widgets.main_window);

        widgets.load_window_size();

        // Get startup window size and propagate so all components have correct narrow/wide layout.
        sender.input(AppMsg::Activate(widgets.main_window.default_width()));

        model.spinner.set_visible(true);
        model.spinner.start();

        model.bootstrap.emit(BootstrapInput::Start);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppMsg::Activate(width) => {
                if width >= 500 {
                    sender.input(AppMsg::Adapt(adaptive::Layout::Wide));
                } else {
                    sender.input(AppMsg::Adapt(adaptive::Layout::Narrow));
                }
            },
            AppMsg::Quit => main_application().quit(),
            AppMsg::Ignore => {
                // info!("Intentionally ignoring a message");
            },
            AppMsg::SettingsChanged(settings) => {
                if let Err(e) = App::save_settings(&settings) {
                    error!("Failed to save settings: {}", e);
                }
            },
            AppMsg::ToggleSidebar => {
                let show = self.main_navigation.shows_sidebar();
                self.main_navigation.set_show_sidebar(!show);
                self.spinner.set_visible(show);
            },
            AppMsg::SwitchView => {
                let child = self.main_stack.visible_child();
                let child_name = self.main_stack.visible_child_name()
                    .and_then(|x| ViewName::from_str(x.as_str()).ok())
                    .unwrap_or(ViewName::Nothing);

                // Set special library header, otherwise set standard label header
                if child_name == ViewName::Library {
                    let vs = adw::ViewSwitcher::builder()
                        .stack(self.library.widget())
                        .policy(adw::ViewSwitcherPolicy::Wide)
                        .build();
                    self.header_bar.set_title_widget(Some(&vs));
                } else if let Some(child) = child {
                    let page = self.main_stack.page(&child);
                    let title = page.title().map(|x| x.to_string());
                    let title = title.map(|text| {
                        gtk::Label::builder()
                            .label(text)
                            .css_classes(["title"])
                            .build()
                    });
                    self.header_bar.set_title_widget(title.as_ref());
                }

                // figure out which view to activate
                match child_name {
                    ViewName::Library | ViewName::All | ViewName::Month | ViewName::Year => {
                        // Note that we'll only won't get All, Month, and Year activations
                        // here, they are handled in the Library view. However, we must handle
                        // the enums for completeness.
                        self.library.emit(LibraryInput::Activate);
                    },
                    ViewName::Videos => self.videos_page.emit(AlbumInput::Activate),
                    ViewName::Selfies => self.selfies_page.emit(AlbumInput::Activate),
                    ViewName::Animated => self.motion_page.emit(AlbumInput::Activate),
                    ViewName::Folders => self.folders_album.emit(FoldersAlbumInput::Activate),
                    ViewName::Folder => self.folder_album.emit(AlbumInput::Activate),
                    ViewName::People => self.people_page.emit(PeopleAlbumInput::Activate),
                    ViewName::Person => self.person_album.emit(PersonAlbumInput::Activate),
                    ViewName::Places => self.places_page.emit(PlacesAlbumInput::Activate),
                    ViewName::Nothing => event!(Level::WARN, "Nothing activated... which should not happen"),
                }
            },
            AppMsg::View(visual_id, filter) => {
                // Send message to show image
                self.view_nav.emit(ViewNavInput::View(visual_id, filter));

                // Display navigation page for viewing an individual photo.
                self.picture_navigation_view.push_by_tag("picture");
            },
            AppMsg::ViewHidden => {
                self.view_nav.emit(ViewNavInput::Hidden);
            },
            AppMsg::ViewFolder(path) => {
                self.folder_album.emit(AlbumInput::Activate);
                self.folder_album.emit(AlbumInput::Filter(AlbumFilter::Folder(path)));
                self.picture_navigation_view.push_by_tag("album");
            },
            AppMsg::ViewGeographicArea(cell_index) => {
                self.folder_album.emit(AlbumInput::Activate);
                self.folder_album.emit(AlbumInput::Filter(AlbumFilter::GeographicArea(cell_index)));
                self.picture_navigation_view.push_by_tag("album");

            },
            AppMsg::ViewPerson(person) => {
                //info!("picture_ids = {:?}", picture_ids);
                info!("Viewing person: {}", person.person_id);
                self.person_album.emit(PersonAlbumInput::Activate);
                self.person_album.emit(PersonAlbumInput::View(person));
                self.picture_navigation_view.push_by_tag("person_album");
            },
            AppMsg::PersonDeleted => {
                self.picture_navigation_view.pop();
                self.people_page.emit(PeopleAlbumInput::Refresh);
            },
            AppMsg::PersonRenamed => {
                self.people_page.emit(PeopleAlbumInput::Refresh);
            },
            AppMsg::TaskStarted(task_name) => {
                self.spinner.start();
                self.spinner.set_visible(!self.main_navigation.shows_sidebar());
                self.banner.set_revealed(true);

                match task_name {
                    TaskName::Scan(MediaType::Photo) => {
                        self.banner.set_title(&fl!("banner-scan-photos"));
                    },
                    TaskName::Scan(MediaType::Video) => {
                        self.banner.set_title(&fl!("banner-scan-videos"));
                    },
                    TaskName::Enrich(MediaType::Photo) => {
                        self.banner.set_title(&fl!("banner-metadata-photos"));
                    },
                    TaskName::Enrich(MediaType::Video) => {
                        self.banner.set_title(&fl!("banner-metadata-videos"));
                    },
                    TaskName::MotionPhoto => {
                        self.banner.set_title(&fl!("banner-extract-motion-photos"));
                    },
                    TaskName::Thumbnail(MediaType::Photo) => {
                        self.banner.set_title(&fl!("banner-thumbnails-photos"));
                    },
                    TaskName::Thumbnail(MediaType::Video) => {
                        self.banner.set_title(&fl!("banner-thumbnails-videos"));
                    },
                    TaskName::DetectFaces => {
                        self.banner.set_title(&fl!("banner-detect-faces-photos"));
                    },
                    TaskName::RecognizeFaces => {
                        self.banner.set_title(&fl!("banner-recognize-faces-photos"));
                    },
                    TaskName::Clean(MediaType::Photo) => {
                        self.banner.set_title(&fl!("banner-clean-photos"));
                    },
                    TaskName::Clean(MediaType::Video) => {
                        self.banner.set_title(&fl!("banner-clean-videos"));
                    },
                };
            },
            AppMsg::BootstrapCompleted => {
                event!(Level::INFO, "Bootstrap completed.");
                self.spinner.stop();
                self.banner.set_revealed(false);
            },
            AppMsg::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                self.video_transcode.emit(VideoTranscodeInput::All);
            },
            AppMsg::ScanPictureForFaces(picture_id) => {
                info!("Scan picture for faces: {}", picture_id);
                self.bootstrap.emit(BootstrapInput::ScanPictureForFaces(picture_id));
            },
            AppMsg::ScanPicturesForFaces => {
                info!("Scan pictures for faces");
                self.bootstrap.emit(BootstrapInput::ScanPicturesForFaces);
            },
            AppMsg::Adapt(adaptive::Layout::Narrow) => {
                self.main_navigation.set_collapsed(true);
                self.main_navigation.set_show_sidebar(false);

                // Notify of a change of layout.
                *self.adaptive_layout.write() = adaptive::Layout::Narrow;
            },
            AppMsg::Adapt(adaptive::Layout::Wide) => {
                let show = self.main_navigation.shows_sidebar();
                self.main_navigation.set_collapsed(false);
                self.main_navigation.set_show_sidebar(show);

                // Notify of a change of layout.
                *self.adaptive_layout.write() = adaptive::Layout::Wide;
            },
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        widgets.save_window_size().unwrap();
    }
}

impl App {
    pub fn load_settings() -> Result<Settings> {
        info!("Loading settings");
        let gio_settings = gio::Settings::new(APP_ID);
        Ok(Settings {
            show_selfies: gio_settings.boolean("show-selfies"),
            face_detection_mode: FaceDetectionMode::from_str(&gio_settings.string("face-detection-mode"))
                .unwrap_or(FaceDetectionMode::Off),
        })
    }

    pub fn save_settings(settings: &Settings) -> Result<()> {
        info!("Saving settings");
        let gio_settings = gio::Settings::new(APP_ID);
        gio_settings.set_boolean("show-selfies", settings.show_selfies)?;
        gio_settings.set_string("face-detection-mode", settings.face_detection_mode.as_ref())?;
        Ok(())
    }
}

impl AppWidgets {
    fn show_selfies() -> bool {
        let settings = gio::Settings::new(APP_ID);
        settings.boolean("show-selfies")
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = gio::Settings::new(APP_ID);
        let (width, height) = self.main_window.default_size();

        settings.set_int("window-width", width)?;
        settings.set_int("window-height", height)?;

        settings.set_boolean("is-maximized", self.main_window.is_maximized())?;

        std::result::Result::Ok(())
    }

    fn load_window_size(&self) {
        let settings = gio::Settings::new(APP_ID);

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.main_window.set_default_size(width, height);

        if is_maximized {
            self.main_window.maximize();
        }
    }
}
