// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{
    actions::{RelmAction, RelmActionGroup},
    adw, gtk, main_application, Component, ComponentController, ComponentParts, ComponentSender,
    Controller, SimpleComponent, WorkerController,
};

use gtk::prelude::{
    ApplicationExt, ApplicationWindowExt, GtkWindowExt, OrientableExt, SettingsExt, WidgetExt,
};
use gtk::{gio, glib};
use relm4::adw::prelude::AdwApplicationWindowExt;

use crate::config::{APP_ID, PROFILE};
use photos_core::repo::PictureId;
use photos_core::YearMonth;
use relm4::adw::prelude::NavigationPageExt;
use std::sync::{Arc, Mutex};
use relm4::gtk::prelude::ButtonExt;


mod components;

use self::components::{
    about::AboutDialog, all_photos::AllPhotos, all_photos::AllPhotosInput,
    all_photos::AllPhotosOutput, month_photos::MonthPhotos, month_photos::MonthPhotosInput,
    month_photos::MonthPhotosOutput, one_photo::OnePhoto, one_photo::OnePhotoInput,
    year_photos::YearPhotos, year_photos::YearPhotosOutput,
    selfie_photos::SelfiePhotos,
};

mod background;

use self::background::{
    scan_photos::ScanPhotos,
    scan_photos::ScanPhotosInput,
    scan_photos::ScanPhotosOutput,
    generate_previews::GeneratePreviews,
    generate_previews::GeneratePreviewsInput,
    generate_previews::GeneratePreviewsOutput,
};

pub(super) struct App {
    scan_photos: WorkerController<ScanPhotos>,
    generate_previews: WorkerController<GeneratePreviews>,
    about_dialog: Controller<AboutDialog>,
    all_photos: Controller<AllPhotos>,
    month_photos: Controller<MonthPhotos>,
    year_photos: Controller<YearPhotos>,
    one_photo: Controller<OnePhoto>,
    selfie_photos: Controller<SelfiePhotos>,

    // Main navigation. Parent of library stack.
    main_navigation: adw::OverlaySplitView,

    // Stack containing Library, Selfies, etc.
    main_stack: gtk::Stack,

    // Library pages
    library_view_stack: adw::ViewStack,

    // Switch between library views and single image view.
    picture_navigation_view: adw::NavigationView,

    // Window header bar
    header_bar: adw::HeaderBar,
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

    // Scroll to first photo in month
    GoToMonth(YearMonth),

    // Scroll to first photo in year
    GoToYear(i32),

    // Photos have been scanned and repo can be updated
    ScanAllCompleted,

    // Preview generation completed
    PreviewsGenerated,
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
                add_setter: (&main_navigation, "show-sidebar", &false.into()),
            },

            // Top-level navigation view containing:
            // 1. Navigation view containing stack of pages.
            // 2. Page for displaying a single photo.
            #[local_ref]
            picture_navigation_view -> adw::NavigationView {
                set_pop_on_escape: true,

                // Page for showing main navigation. Such as "Library", "Selfies", etc.
                adw::NavigationPage {
                    #[local_ref]
                    main_navigation -> adw::OverlaySplitView {

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
                                set_content = &gtk::StackSidebar {
                                    set_stack: &main_stack,
                                },
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
                                    //    set_stack: Some(&library_view_stack),
                                    //    set_policy: adw::ViewSwitcherPolicy::Wide,
                                    //},
                                },

                                // NOTE I would like this to be an adw::ViewStack
                                // so that I could use a adw::ViewSwitcher in the sidebar
                                // that would show icons.
                                // However, adw::ViewSwitch can't display vertically.
                                #[wrap(Some)]
                                #[local_ref]
                                set_content = &main_stack -> gtk::Stack {
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
                                        set_title: "Selfies",
                                        set_name: "Selfies",
                                        // NOTE gtk::StackSidebar doesn't show icon :-/
                                        set_icon_name: "sentiment-very-satisfied-symbolic",
                                    },
                                },
                            },
                        },
                    },
                },

                // Page for showing a single photo.
                adw::NavigationPage {
                    set_tag: Some("picture"),
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

        // TODO use XDG_PICTURES_DIR as the default, but let users override in preferences.
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

        //let controller = photos_core::Controller::new(scan.clone(), repo, previewer);
        //let controller = Arc::new(Mutex::new(controller));

        let scan_photos = ScanPhotos::builder()
            .detach_worker((scan.clone(), repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                ScanPhotosOutput::ScanAllCompleted => AppMsg::ScanAllCompleted,
            });

        let generate_previews = GeneratePreviews::builder()
            .detach_worker((scan.clone(), previewer.clone(), repo.clone()))
            .forward(sender.input_sender(), |msg| match msg {
                GeneratePreviewsOutput::PreviewsGenerated => AppMsg::PreviewsGenerated,
            });

        let all_photos = AllPhotos::builder()
            .launch(repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                AllPhotosOutput::PhotoSelected(id) => AppMsg::ViewPhoto(id),
            });

        let month_photos = MonthPhotos::builder()
            .launch(repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                MonthPhotosOutput::MonthSelected(ym) => AppMsg::GoToMonth(ym),
            });

        let year_photos = YearPhotos::builder()
            .launch(repo.clone())
            .forward(sender.input_sender(), |msg| match msg {
                YearPhotosOutput::YearSelected(year) => AppMsg::GoToYear(year),
            });

        let one_photo = OnePhoto::builder()
            .launch((scan.clone(), repo.clone()))
            .detach();

        let selfie_photos = SelfiePhotos::builder()
            .launch(repo.clone())
            .detach();

        let about_dialog = AboutDialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();


        let library_view_stack = adw::ViewStack::new();

        let picture_navigation_view = adw::NavigationView::builder().build();

        let main_navigation = adw::OverlaySplitView::builder().build();

        let main_stack = gtk::Stack::new();

        let header_bar = adw::HeaderBar::new();

        let model = Self {
            scan_photos,
            generate_previews,
            about_dialog,
            all_photos,
            month_photos,
            year_photos,
            one_photo,
            selfie_photos,
            main_navigation: main_navigation.clone(),
            main_stack: main_stack.clone(),
            library_view_stack: library_view_stack.clone(),
            picture_navigation_view: picture_navigation_view.clone(),
            header_bar: header_bar.clone(),
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

        model.scan_photos.sender().emit(ScanPhotosInput::ScanAll);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => main_application().quit(),
            AppMsg::ToggleSidebar => {
                let show = self.main_navigation.shows_sidebar();
                self.main_navigation.set_show_sidebar(!show);
            },
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
                    let page =self.main_stack.page(&child);
                    let title = page.title().map(|x| x.to_string());
                    let label = gtk::Label::builder()
                        .label(title.unwrap_or("-".to_string()))
                        .css_classes(["title"])
                        .build();
                    self.header_bar.set_title_widget(Some(&label));
                }
            },
            AppMsg::ViewPhoto(picture_id) => {
                // Send message to OnePhoto to show image
                self.one_photo.emit(OnePhotoInput::ViewPhoto(picture_id));

                // Display navigation page for viewing an individual photo.
                self.picture_navigation_view.push_by_tag("picture");
            },
            AppMsg::GoToMonth(ym) => {
                // Display all photos view.
                self.library_view_stack.set_visible_child_name("all");
                self.all_photos.emit(AllPhotosInput::GoToMonth(ym));
            },
            AppMsg::GoToYear(year) => {
                // Display month photos view.
                self.library_view_stack.set_visible_child_name("month");
                self.month_photos.emit(MonthPhotosInput::GoToYear(year));
            },
            AppMsg::ScanAllCompleted => {
                println!("Scan all completed msg received.");
                self.generate_previews.emit(GeneratePreviewsInput::Generate);
            },
            AppMsg::PreviewsGenerated => {
                println!("Previews generated completed.");
            },
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        widgets.save_window_size().unwrap();
    }
}

impl AppWidgets {
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
