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
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent, WorkerController,
};

use crate::config::{APP_ID, PROFILE};
use photos_core::repo::PictureId;
use photos_core::YearMonth;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

mod components;

use self::components::{
    about::AboutDialog,
    album::{Album, AlbumFilter, AlbumInput, AlbumOutput},
    folder_photos::{FolderPhotos, FolderPhotosInput, FolderPhotosOutput},
    month_photos::{MonthPhotos, MonthPhotosInput, MonthPhotosOutput},
    one_photo::{OnePhoto, OnePhotoInput},
    year_photos::{YearPhotos, YearPhotosInput, YearPhotosOutput},
};

mod background;

use self::background::{
    cleanup::{Cleanup, CleanupInput, CleanupOutput},
    generate_previews::{GeneratePreviews, GeneratePreviewsInput, GeneratePreviewsOutput},
    scan_photos::{ScanPhotos, ScanPhotosInput, ScanPhotosOutput},
};

pub(super) struct App {
    scan_photos: WorkerController<ScanPhotos>,
    generate_previews: AsyncController<GeneratePreviews>,
    cleanup: WorkerController<Cleanup>,

    about_dialog: Controller<AboutDialog>,
    all_photos: AsyncController<Album>,
    month_photos: AsyncController<MonthPhotos>,
    year_photos: AsyncController<YearPhotos>,
    one_photo: AsyncController<OnePhoto>,

    show_selfies: bool,
    selfie_photos: AsyncController<Album>,

    // Grid of folders of photos
    folder_photos: AsyncController<FolderPhotos>,

    // Folder album currently being viewed
    folder_album: AsyncController<Album>,

    // Main navigation. Parent of library stack.
    main_navigation: adw::OverlaySplitView,

    // Stack containing Library, Selfies, Folders, etc.
    main_stack: gtk::Stack,

    // Library pages
    library_view_stack: adw::ViewStack,

    // Switch between library views and single image view.
    picture_navigation_view: adw::NavigationView,

    // Window header bar
    header_bar: adw::HeaderBar,

    // Activity indicator. Only shown when progress bar is hidden.
    spinner: gtk::Spinner,

    // TODO there are too many progress_* fields. Move to a custom Progress component?

    // Progress indicator.
    progress_bar: gtk::ProgressBar,

    // Container for related progress bar components
    progress_box: gtk::Box,

    // Expected number of items we are recording progress for
    progress_end_count: usize,

    // Number of items processed so far.
    progress_current_count: usize,

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

    // Show photo for ID.
    ViewPhoto(PictureId),

    ViewFolder(PathBuf),

    // Scroll to first photo in month
    GoToMonth(YearMonth),

    // Scroll to first photo in year
    GoToYear(i32),

    // Refresh all photo grid views. Manually triggered by button press.
    Refresh,

    // File-system scan events
    ScanStarted,
    ScanCompleted,

    // Thumbnail generation events
    ThumbnailGenerationStarted(usize),
    ThumbnailGenerated,
    ThumbnailGenerationCompleted,

    // Cleanup events
    CleanupStarted(usize),
    CleanupCleaned,
    CleanupCompleted,
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
                "_About Photo Romantic" => AboutAction,
            }
        }
    }

    view! {
        main_window = adw::ApplicationWindow::new(&main_application()) {
            set_visible: true,
            set_width_request: 400,
            set_height_request: 400,

            connect_close_request[sender] => move |_| {
                sender.input(AppMsg::Quit);
                glib::Propagation::Stop
            },

            #[wrap(Some)]
            set_help_overlay: shortcuts = &gtk::Builder::from_resource(
                    "/dev/romantics/Photos/gtk/help-overlay.ui"
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
                                    #[local_ref]
                                    progress_box -> gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_margin_all: 12,

                                        #[local_ref]
                                        progress_bar -> gtk::ProgressBar {
                                            set_show_text: true,
                                        },
                                    }
                                }
                            }
                        },

                        #[wrap(Some)]
                        set_content = &adw::NavigationPage {
                            set_title: "-",
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
                                    //    set_stack: Some(&library_view_stack),
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
                                        connect_button_clicked => AppMsg::Refresh,
                                    },

                                    #[local_ref]
                                    main_stack -> gtk::Stack {
                                        connect_visible_child_notify => AppMsg::SwitchView,

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,

                                            #[local_ref]
                                            library_view_stack -> adw::ViewStack {
                                                add_titled_with_icon[Some("all"), "All", "playlist-infinite-symbolic"] = model.all_photos.widget(),
                                                add_titled_with_icon[Some("month"), "Month", "month-symbolic"] = model.month_photos.widget(),
                                                add_titled_with_icon[Some("year"), "Year", "year-symbolic"] = model.year_photos.widget(),
                                            },

                                            #[name(switcher_bar)]
                                            adw::ViewSwitcherBar {
                                                set_stack: Some(&library_view_stack),
                                            },
                                        } -> {
                                            set_title: "Library",
                                            set_name: "Library",

                                            // NOTE gtk::StackSidebar doesn't show icon :-/
                                            set_icon_name: "image-alt-symbolic",
                                        },

                                        add_child = &gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,
                                            container_add: model.selfie_photos.widget(),
                                        } -> {
                                            set_visible: model.show_selfies,
                                            set_title: "Selfies",
                                            set_name: "Selfies",
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
                                            set_name: "Folders",
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
                    set_title: "-",
                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar {
                            #[wrap(Some)]
                            set_title_widget = &gtk::Label {
                                set_label: "Folder", // TODO set title to folder name
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
                    set_title: "-",
                    model.one_photo.widget(),
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let data_dir = glib::user_data_dir().join("photo-romantic");
        let _ = std::fs::create_dir_all(&data_dir);

        let cache_dir = glib::user_cache_dir().join("photo-romantic");
        let _ = std::fs::create_dir_all(&cache_dir);

        let pic_base_dir = glib::user_special_dir(glib::enums::UserDirectory::Pictures)
            .expect("Expect XDG_PICTURES_DIR");

        let repo = {
            let db_path = data_dir.join("pictures.sqlite");
            photos_core::Repository::open(&pic_base_dir, &db_path).unwrap()
        };

        let scan = photos_core::Scanner::build(&pic_base_dir).unwrap();

        let previewer = {
            let preview_base_path = cache_dir.join("previews");
            let _ = std::fs::create_dir_all(&preview_base_path);
            photos_core::Previewer::build(&preview_base_path).unwrap()
        };

        let repo = Arc::new(Mutex::new(repo));

        let scan_photos = ScanPhotos::builder()
            .detach_worker((scan.clone(), repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ScanPhotosOutput::Started => AppMsg::ScanStarted,
                ScanPhotosOutput::Completed => AppMsg::ScanCompleted,
            });

        let generate_previews = GeneratePreviews::builder()
            .launch((previewer.clone(), repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                GeneratePreviewsOutput::Started(count) => AppMsg::ThumbnailGenerationStarted(count),
                GeneratePreviewsOutput::Generated => AppMsg::ThumbnailGenerated,
                GeneratePreviewsOutput::Completed => AppMsg::ThumbnailGenerationCompleted,
            });

        let cleanup =
            Cleanup::builder()
                .detach_worker(repo.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    CleanupOutput::Started(count) => AppMsg::CleanupStarted(count),
                    CleanupOutput::Cleaned => AppMsg::CleanupCleaned,
                    CleanupOutput::Completed => AppMsg::CleanupCompleted,
                });

        let all_photos = Album::builder()
            .launch((repo.clone(), AlbumFilter::All))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::PhotoSelected(id) => AppMsg::ViewPhoto(id),
            });

        let month_photos =
            MonthPhotos::builder()
                .launch(repo.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    MonthPhotosOutput::MonthSelected(ym) => AppMsg::GoToMonth(ym),
                });

        let year_photos =
            YearPhotos::builder()
                .launch(repo.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    YearPhotosOutput::YearSelected(year) => AppMsg::GoToYear(year),
                });

        let one_photo = OnePhoto::builder()
            .launch((scan.clone(), repo.clone()))
            .detach();

        let selfie_photos = Album::builder()
            .launch((repo.clone(), AlbumFilter::Selfies))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::PhotoSelected(id) => AppMsg::ViewPhoto(id),
            });

        let show_selfies = AppWidgets::show_selfies();

        let folder_photos =
            FolderPhotos::builder()
                .launch(repo.clone())
                .forward(sender.input_sender(), |msg| match msg {
                    FolderPhotosOutput::FolderSelected(path) => AppMsg::ViewFolder(path),
                });

        let folder_album = Album::builder()
            .launch((repo.clone(), AlbumFilter::None))
            .forward(sender.input_sender(), |msg| match msg {
                AlbumOutput::PhotoSelected(id) => AppMsg::ViewPhoto(id),
            });

        folder_album.emit(AlbumInput::Refresh); // initial photo

        let about_dialog = AboutDialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let library_view_stack = adw::ViewStack::new();

        let picture_navigation_view = adw::NavigationView::builder().build();

        let main_navigation = adw::OverlaySplitView::builder().build();

        let main_stack = gtk::Stack::new();

        let header_bar = adw::HeaderBar::new();

        let spinner = gtk::Spinner::builder().visible(false).build();

        let progress_bar = gtk::ProgressBar::builder().pulse_step(0.05).build();

        let progress_box = gtk::Box::builder().build();

        let banner = adw::Banner::new("-");

        let model = Self {
            scan_photos,
            generate_previews,
            cleanup,
            about_dialog,
            all_photos,
            month_photos,
            year_photos,
            one_photo,
            selfie_photos,
            show_selfies,
            folder_photos,
            folder_album,
            main_navigation: main_navigation.clone(),
            main_stack: main_stack.clone(),
            library_view_stack: library_view_stack.clone(),
            picture_navigation_view: picture_navigation_view.clone(),
            header_bar: header_bar.clone(),
            spinner: spinner.clone(),
            progress_bar: progress_bar.clone(),
            progress_box: progress_box.clone(),
            progress_end_count: 0,
            progress_current_count: 0,
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

        actions.add_action(shortcuts_action);
        actions.add_action(about_action);
        actions.register_for_widget(&widgets.main_window);

        widgets.load_window_size();

        model.all_photos.emit(AlbumInput::Refresh);

        model.scan_photos.sender().emit(ScanPhotosInput::Start);

        //        model.selfie_photos.emit(SelfiePhotosInput::Refresh);
        //      model.month_photos.emit(MonthPhotosInput::Refresh);
        //    model.year_photos.emit(YearPhotosInput::Refresh);

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
                let child_name = self.main_stack.visible_child_name();

                if child_name.is_some_and(|x| x.as_str() == "Library") {
                    let vs = adw::ViewSwitcher::builder()
                        .stack(&self.library_view_stack)
                        .policy(adw::ViewSwitcherPolicy::Wide)
                        .build();
                    self.header_bar.set_title_widget(Some(&vs));
                } else if let Some(child) = child {
                    let page = self.main_stack.page(&child);
                    let title = page.title().map(|x| x.to_string());
                    let label = gtk::Label::builder()
                        .label(title.unwrap_or("-".to_string()))
                        .css_classes(["title"])
                        .build();
                    self.header_bar.set_title_widget(Some(&label));
                }
            }
            AppMsg::ViewPhoto(picture_id) => {
                // Send message to OnePhoto to show image
                self.one_photo.emit(OnePhotoInput::ViewPhoto(picture_id));

                // Display navigation page for viewing an individual photo.
                self.picture_navigation_view.push_by_tag("picture");
            }
            AppMsg::ViewFolder(path) => {
                self.folder_album
                    .emit(AlbumInput::Filter(AlbumFilter::Folder(path)));
                //self.folder_album
                self.picture_navigation_view.push_by_tag("album");
            }
            AppMsg::GoToMonth(ym) => {
                // Display all photos view.
                self.library_view_stack.set_visible_child_name("all");
                self.all_photos.emit(AlbumInput::GoToMonth(ym));
            }
            AppMsg::GoToYear(year) => {
                // Display month photos view.
                self.library_view_stack.set_visible_child_name("month");
                self.month_photos.emit(MonthPhotosInput::GoToYear(year));
            }
            AppMsg::Refresh => {
                println!("Refresh photo grids");

                // Refresh messages cause the photos to be loaded into various photo grids
                // TODO can we just refresh the currently visible photo grid?
                self.all_photos.emit(AlbumInput::Refresh);
                self.selfie_photos.emit(AlbumInput::Refresh);
                self.folder_photos.emit(FolderPhotosInput::Refresh);
                self.month_photos.emit(MonthPhotosInput::Refresh);
                self.year_photos.emit(YearPhotosInput::Refresh);
            }
            AppMsg::ScanStarted => {
                self.spinner.start();
                self.banner.set_title("Scanning file system for photos.");
                self.banner.set_revealed(true);
            }
            AppMsg::ScanCompleted => {
                println!("Scan all completed msg received.");

                //self.generate_previews.emit(GeneratePreviewsInput::Start);
                self.cleanup.emit(CleanupInput::Start);
            }
            AppMsg::ThumbnailGenerationStarted(count) => {
                println!("Thumbnail generation started.");
                self.banner
                    .set_title("Generating thumbnails. This will take a while.");
                // Show button to refresh all photo grids.
                self.banner.set_button_label(Some("Refresh"));
                self.banner.set_revealed(true);

                self.spinner.start();

                let show = self.main_navigation.shows_sidebar();
                self.spinner.set_visible(!show);

                self.progress_end_count = count;
                self.progress_current_count = 0;

                self.progress_box.set_visible(true);
                self.progress_bar.set_fraction(0.0);
                self.progress_bar.set_text(Some("Generating thumbnails."));
                self.progress_bar.set_pulse_step(0.25);
            }
            AppMsg::ThumbnailGenerated => {
                println!("Thumbnail generated.");
                self.progress_current_count += 1;
                // Show pulsing for first 20 items so that it catches the eye, then
                // switch to fractional view
                if self.progress_current_count < 20 {
                    self.progress_bar.pulse();
                } else {
                    if self.progress_current_count == 20 {
                        self.progress_bar.set_text(None);
                    }
                    let fraction =
                        self.progress_current_count as f64 / self.progress_end_count as f64;
                    self.progress_bar.set_fraction(fraction);
                }
            }
            AppMsg::ThumbnailGenerationCompleted => {
                println!("Thumbnail generation completed.");
                self.spinner.stop();
                self.banner.set_revealed(false);
                self.banner.set_button_label(None);
                self.progress_box.set_visible(false);

                // Refresh messages cause the photos to be loaded into various photo grids
                self.all_photos.emit(AlbumInput::Refresh);
                self.selfie_photos.emit(AlbumInput::Refresh);
                self.folder_photos.emit(FolderPhotosInput::Refresh);
                self.month_photos.emit(MonthPhotosInput::Refresh);
                self.year_photos.emit(YearPhotosInput::Refresh);
            }
            AppMsg::CleanupCompleted => {
                println!("Cleanup completed.");
                self.spinner.stop();
                self.banner.set_revealed(false);
                self.banner.set_button_label(None);
                self.progress_box.set_visible(false);

                self.generate_previews.emit(GeneratePreviewsInput::Start);
            }

            AppMsg::CleanupStarted(count) => {
                println!("Cleanup started.");
                self.banner.set_title("Database maintenance.");
                // Show button to refresh all photo grids.
                //self.banner.set_button_label(Some("Refresh"));
                self.banner.set_revealed(true);

                self.spinner.start();

                let show = self.main_navigation.shows_sidebar();
                self.spinner.set_visible(!show);

                self.progress_end_count = count;
                self.progress_current_count = 0;

                self.progress_box.set_visible(true);
                self.progress_bar.set_fraction(0.0);
                self.progress_bar.set_text(Some("Database maintenance"));
                self.progress_bar.set_pulse_step(0.25);
            }
            AppMsg::CleanupCleaned => {
                println!("Cleanup cleaned.");
                self.progress_current_count += 1;
                // Show pulsing for first 1000 items so that it catches the eye, then
                // switch to fractional view
                if self.progress_current_count < 1000 {
                    self.progress_bar.pulse();
                } else {
                    if self.progress_current_count == 1000 {
                        self.progress_bar.set_text(None);
                    }
                    let fraction =
                        self.progress_current_count as f64 / self.progress_end_count as f64;
                    self.progress_bar.set_fraction(fraction);
                }
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
