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
            ApplicationExt, ApplicationWindowExt, ButtonExt, GtkWindowExt, OrientableExt,
            SettingsExt, WidgetExt,
        },
    },
    main_application,
    prelude::AsyncController,
    Component, ComponentController, ComponentParts, ComponentSender, Controller,
    SimpleComponent, WorkerController,
    shared_state::Reducer,
};

use relm4;

use crate::config::{APP_ID, PROFILE};
use fotema_core::database;
use fotema_core::video;
use fotema_core::VisualId;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::str::FromStr;

use strum::EnumString;
use strum::IntoStaticStr;

use tracing::{event, Level};

mod components;

use self::components::{
    about::AboutDialog,
    album::{Album, AlbumInput, AlbumOutput},
    album_filter::AlbumFilter,
    folder_photos::{FolderPhotos, FolderPhotosInput, FolderPhotosOutput},
    library::{Library, LibraryInput, LibraryOutput},
    viewer::{Viewer, ViewerInput, ViewerOutput},
    preferences::{PreferencesDialog, PreferencesInput, PreferencesOutput},
};

mod background;

use self::background::{
    bootstrap::{Bootstrap, BootstrapInput, BootstrapOutput},
    video_transcode::{VideoTranscode, VideoTranscodeInput},
};

use self::components::progress_monitor::ProgressMonitor;
use self::components::progress_panel::ProgressPanel;

// Visual items to be shared between various views.
// State is loaded by the `load_library` background task.
type SharedState = Arc<relm4::SharedState<Vec<Arc<fotema_core::Visual>>>>;

/// Name of a view that can be displayed
#[derive(Copy, Clone, Debug, Eq, PartialEq, EnumString, IntoStaticStr)]
pub enum ViewName {
    Nothing, // no view
    Library, // parent of all, month, and year views.
    All,
    Month,
    Year,
    Videos,
    Animated,
    Folders,
    Folder,
    Selfies,
}

impl Default for ViewName {
    fn default() -> Self { ViewName::Nothing }
}

/// Currently visible view
/// This allows a view to know if it is visible or not and to lazily load
/// images into the photo grids. Without lazy loading Fotema will take too long to
/// update its views and GNOME will tell the user "Fotema is not responding" and offer
/// to kill the app :-(
type ActiveView = Arc<relm4::SharedState<ViewName>>;

pub(super) struct App {
    about_dialog: Controller<AboutDialog>,
    preferences_dialog: Controller<PreferencesDialog>,

    bootstrap: WorkerController<Bootstrap>,
    video_transcode: WorkerController<VideoTranscode>,

    library: Controller<Library>,

    viewer: AsyncController<Viewer>,

    show_selfies: bool,
    selfies_page: Controller<Album>,
    videos_page: Controller<Album>,
    motion_page: Controller<Album>,

    // Grid of folders of photos
    folder_photos: Controller<FolderPhotos>,

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
    Quit,

    // Toggle visibility of sidebar
    ToggleSidebar,

    // A sidebar item has been clicked
    SwitchView,

    // Show item.
    View(VisualId, AlbumFilter),

    // Shown item is dismissed.
    ViewHidden,

    ViewFolder(PathBuf),

    // A task has started.
    TaskStarted(String),

    // Preferences
    PreferencesUpdated,

    // All background bootstrap tasks have completed
    BootstrapCompleted,

    TranscodeAll,
}

relm4::new_action_group!(pub(super) WindowActionGroup, "win");
relm4::new_stateless_action!(PreferencesAction, WindowActionGroup, "preferences");
relm4::new_stateless_action!(pub(super) ShortcutsAction, WindowActionGroup, "show-help-overlay");
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
                "_Preferences" => PreferencesAction,
                "_Keyboard" => ShortcutsAction,
                "_About Fotema" => AboutAction,
            }
        }
    }

    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_visible: true,

            // See https://linuxphoneapps.org/docs/resources/developer-information/#hardware-specs-to-consider
            set_width_request: 360,
            set_height_request: 294,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

            #[wrap(Some)]
            set_help_overlay: shortcuts = &gtk::Builder::from_resource(
                    "/app/fotema/Fotema/gtk/help-overlay.ui"
                )
                .object::<gtk::ShortcutsWindow>("help_overlay")
                .unwrap() -> gtk::ShortcutsWindow {
                    set_transient_for: Some(&main_window),
                    set_application: Some(&main_application()),
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
                add_setter: (&header_bar, "show-title", &false.into()),
                add_setter: (&switcher_bar, "reveal", &true.into()),
                add_setter: (&main_navigation, "collapsed", &true.into()),
                //add_setter: (&main_navigation, "show-sidebar", &false.into()),
                add_setter: (&spinner, "visible", &true.into()),
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
                    set_title: "Main Navigation",

                    #[local_ref]
                    main_navigation -> adw::OverlaySplitView {

                        set_max_sidebar_width: 200.0,

                        #[wrap(Some)]
                        set_sidebar = &adw::NavigationPage {
                            adw::ToolbarView {
                                add_top_bar = &adw::HeaderBar {
                                    #[wrap(Some)]
                                    set_title_widget = &gtk::Label {
                                        set_label: "Photos",
                                        add_css_class: "title",
                                    },

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

                                    //#[wrap(Some)]
                                    //set_title_widget = &adw::ViewSwitcher {
                                    //   set_stack: Some(model.library.widget()),
                                    //    set_policy: adw::ViewSwitcherPolicy::Wide,
                                    //},

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
                                            set_title: "Library",
                                            set_name: ViewName::Library.into(),

                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "image-alt-symbolic",
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.videos_page.widget(),
                                        } -> {
                                            set_title: "Videos",
                                            set_name: ViewName::Videos.into(),
                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "video-reel-symbolic",
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.motion_page.widget(),
                                        } -> {
                                            set_title: "Animated",
                                            set_name: ViewName::Animated.into(),
                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "sonar-symbolic",
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.selfies_page.widget(),
                                        } -> {
                                            set_visible: model.show_selfies,
                                            set_title: "Selfies",
                                            set_name: ViewName::Selfies.into(),
                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "sentiment-very-satisfied-symbolic",
                                        },

                                        add_child = &adw::NavigationView {
                                            set_pop_on_escape: true,

                                            adw::NavigationPage {
                                                //set_tag: Some("folders"),
                                                //set_title: "Folder",
                                                model.folder_photos.widget(),
                                            },
                                        } -> {
                                            set_title: "Folders",
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
                                set_label: "Folder",
                                add_css_class: "title",
                            }
                        },

                        #[wrap(Some)]
                        set_content = model.folder_album.widget(),
                    }
                },

                // Page for showing a single photo.
                adw::NavigationPage {
                    set_tag: Some("picture"),
                    model.viewer.widget(),
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

        let db_path = data_dir.join("pictures.sqlite");

        let con = database::setup(&db_path).expect("Must be able to open database");
        let con = Arc::new(Mutex::new(con));

        let video_repo = {
            video::Repository::open(&pic_base_dir, &cache_dir, con.clone()).unwrap()
        };

        let state = SharedState::new(relm4::SharedState::new());
        let active_view = ActiveView::new(relm4::SharedState::new());

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
            .detach_worker((con.clone(), state.clone(), bootstrap_progress_monitor))
            .forward(sender.input_sender(), |msg| match msg {
                BootstrapOutput::TaskStarted(msg) => AppMsg::TaskStarted(msg),
                BootstrapOutput::Completed => AppMsg::BootstrapCompleted,
            });

        let library = Library::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                LibraryOutput::View(id) => AppMsg::View(id, AlbumFilter::All),
            });

        let transcoder = video::Transcoder::new(&cache_dir);

        let video_transcode = VideoTranscode::builder()
            .detach_worker((state.clone(), video_repo, transcoder.clone(), transcode_progress_monitor.clone()))
            .detach();

        let viewer = Viewer::builder()
            .launch((state.clone(), transcode_progress_monitor.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ViewerOutput::TranscodeAll => AppMsg::TranscodeAll,
            });

        let selfies_page = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Selfies, AlbumFilter::Selfies))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
            });

        state.subscribe(selfies_page.sender(), |_| AlbumInput::Refresh);

        let show_selfies = AppWidgets::show_selfies();

        let motion_page = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Animated, AlbumFilter::Motion))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
            });

        state.subscribe(motion_page.sender(), |_| AlbumInput::Refresh);

        let videos_page = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Videos, AlbumFilter::Videos))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
            });

        state.subscribe(videos_page.sender(), |_| AlbumInput::Refresh);

        let folder_photos = FolderPhotos::builder()
            .launch((state.clone(), active_view.clone()))
            .forward(
            sender.input_sender(),
            |msg| match msg {
                FolderPhotosOutput::FolderSelected(path) => AppMsg::ViewFolder(path),
            },
        );

        state.subscribe(folder_photos.sender(), |_| FolderPhotosInput::Refresh);

        let folder_album = Album::builder()
            .launch((state.clone(), active_view.clone(), ViewName::Folder, AlbumFilter::None))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::Selected(id, filter) => AppMsg::View(id, filter),
            });

        state.subscribe(folder_album.sender(), |_| AlbumInput::Refresh);

        let about_dialog = AboutDialog::builder().launch(root.clone()).detach();

        let preferences_dialog = PreferencesDialog::builder().launch(root.clone()).forward(
            sender.input_sender(),
            |msg| match msg {
                PreferencesOutput::Updated => AppMsg::PreferencesUpdated,
            },
        );

        let picture_navigation_view = adw::NavigationView::builder().build();

        let main_navigation = adw::OverlaySplitView::builder().build();

        let main_stack = gtk::Stack::new();

        let header_bar = adw::HeaderBar::new();

        let spinner = gtk::Spinner::builder().visible(false).build();

        let banner = adw::Banner::new("-");

        let model = Self {
            bootstrap,
            video_transcode,

            about_dialog,
            preferences_dialog,

            library,

            viewer,
            motion_page,
            videos_page,
            selfies_page,
            show_selfies,
            folder_photos,
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

        let shortcuts_action = {
            let shortcuts = widgets.shortcuts.clone();
            RelmAction::<ShortcutsAction>::new_stateless(move |_| {
                shortcuts.present();
            })
        };

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

        actions.add_action(shortcuts_action);
        actions.add_action(about_action);
        actions.add_action(preferences_action);

        actions.register_for_widget(&widgets.main_window);

        widgets.load_window_size();

        model.spinner.set_visible(true);
        model.spinner.start();

        model.bootstrap.emit(BootstrapInput::Start);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => main_application().quit(),
            AppMsg::ToggleSidebar => {
                let show = self.main_navigation.shows_sidebar();
                self.main_navigation.set_show_sidebar(!show);
                self.spinner.set_visible(show);
            }
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
                    ViewName::Folders => self.folder_photos.emit(FolderPhotosInput::Activate),
                    ViewName::Folder => self.folder_album.emit(AlbumInput::Activate),
                    ViewName::Nothing => event!(Level::WARN, "Nothing activated... which should not happen"),
                }
            }
            AppMsg::View(visual_id, filter) => {
                // Send message to show image
                self.viewer.emit(ViewerInput::View(visual_id, filter));

                // Display navigation page for viewing an individual photo.
                self.picture_navigation_view.push_by_tag("picture");
            }
            AppMsg::ViewHidden => {
                self.viewer.emit(ViewerInput::Hidden);
            }
            AppMsg::ViewFolder(path) => {
                self.folder_album.emit(AlbumInput::Activate);
                self.folder_album.emit(AlbumInput::Filter(AlbumFilter::Folder(path)));
                self.picture_navigation_view.push_by_tag("album");

            }
            AppMsg::TaskStarted(msg) => {
                self.spinner.start();
                self.spinner.set_visible(!self.main_navigation.shows_sidebar());
                self.banner.set_title(&msg);
                self.banner.set_revealed(true);
            }
            AppMsg::BootstrapCompleted => {
                event!(Level::INFO, "Bootstrap completed.");
                self.spinner.stop();
                self.banner.set_revealed(false);
            }
            AppMsg::TranscodeAll => {
                event!(Level::INFO, "Transcode all");
                self.video_transcode.emit(VideoTranscodeInput::All);
            },

            AppMsg::PreferencesUpdated => {
                event!(Level::INFO, "Preferences updated.");
                // TODO create a Preferences struct to hold preferences and send with update message.
                self.show_selfies = AppWidgets::show_selfies();
            }
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        widgets.save_window_size().unwrap();
    }
}

impl AppWidgets {
    fn show_selfies() -> bool {
        let settings = gio::Settings::new(APP_ID);
        let show_selfies = settings.boolean("show-selfies");
        show_selfies
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let settings = gio::Settings::new(APP_ID);
        let (width, height) = self.main_window.default_size();

        settings.set_int("window-width", width)?;
        settings.set_int("window-height", height)?;

        settings.set_boolean("is-maximized", self.main_window.is_maximized())?;

        Ok(())
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
