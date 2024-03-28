// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::{
    actions::{RelmAction, RelmActionGroup},
    adw, gtk, main_application, Component, ComponentController, ComponentParts, ComponentSender,
    Controller, SimpleComponent,
};

use gtk::prelude::{
    ApplicationExt, ApplicationWindowExt, GtkWindowExt, OrientableExt, SettingsExt, WidgetExt,
};
use gtk::{gio, glib};
use relm4::adw::prelude::AdwApplicationWindowExt;

use crate::config::{APP_ID, PROFILE};
use photos_core::repo::PictureId;
use relm4::adw::prelude::NavigationPageExt;
use std::cell::RefCell;
use std::rc::Rc;


mod components;

use self::{
    components::{
        about::AboutDialog,
        all_photos::AllPhotos,
        all_photos::PhotoGridOutput,
        month_photos::MonthPhotos,
        year_photos::YearPhotos,
    }
};

pub(super) struct App {
    controller: Rc<RefCell<photos_core::Controller>>,
    about_dialog: Controller<AboutDialog>,
    all_photos: Controller<AllPhotos>,
    month_photos: Controller<MonthPhotos>,
    year_photos: Controller<YearPhotos>,
    picture_navigation_view: adw::NavigationView,
    picture_view: gtk::Picture,
}

#[derive(Debug)]
pub(super) enum AppMsg {
    Quit,

    // Show picture for ID.
    ViewPhoto(PictureId),
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
            },

            #[local_ref]
            picture_navigation_view -> adw::NavigationView {
                set_pop_on_escape: true,

                adw::NavigationPage {
                    set_tag: Some("time_period_views"),
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        #[name(header_bar)]
                        adw::HeaderBar {
                            #[wrap(Some)]
                            set_title_widget = &adw::ViewSwitcher {
                                set_stack: Some(&stack),
                            },

                            pack_end = &gtk::MenuButton {
                                set_icon_name: "open-menu-symbolic",
                                set_menu_model: Some(&primary_menu),
                            }
                        },

                        #[name(stack)]
                        adw::ViewStack {
                            add_titled_with_icon[None, "All", "playlist-infinite-symbolic"] = model.all_photos.widget(),
                            add_titled_with_icon[None, "Month", "month-symbolic"] = model.month_photos.widget(),
                            add_titled_with_icon[None, "Year", "year-symbolic"] = model.year_photos.widget(),
                        },

                        #[name(switcher_bar)]
                        adw::ViewSwitcherBar {
                            set_stack: Some(&stack),
                        }
                    },
                },

                adw::NavigationPage {
                    set_tag: Some("picture"),

                    adw::ToolbarView {
                        add_top_bar = &adw::HeaderBar,

                        #[wrap(Some)]
                        set_content = &gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,

                            #[local_ref]
                            picture_view -> gtk::Picture {
                                set_can_shrink: true,
                                set_valign: gtk::Align::Center,
                            }
                        }
                    },
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

        let mut controller = photos_core::Controller::new(scan, repo, previewer);

        // Time consuming!
        match controller.scan() {
            Err(e) => {
                println!("Failed scanning: {:?}", e);
            }
            _ => {}
        }

        let controller = Rc::new(RefCell::new(controller));

        {
            //let result = controller.borrow_mut().update_previews();
            //println!("preview result: {:?}", result);
        }

        let all_photos = AllPhotos::builder()
            .launch(controller.clone())
            .forward(sender.input_sender(), convert_all_photos_output);

        let month_photos = MonthPhotos::builder().launch(controller.clone()).detach();
        let year_photos = YearPhotos::builder().launch(controller.clone()).detach();

        let about_dialog = AboutDialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let picture_view = gtk::Picture::new();

        let picture_navigation_view = adw::NavigationView::builder().build();

        let model = Self {
            controller,
            about_dialog,
            all_photos,
            month_photos,
            year_photos,
            picture_navigation_view: picture_navigation_view.clone(),
            picture_view: picture_view.clone(),
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

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::Quit => main_application().quit(),
            AppMsg::ViewPhoto(picture_id) => {
                println!("Showing photo for {}", picture_id);
                let result = self.controller.borrow_mut().get(picture_id);
                if let Ok(Some(pic)) = result {
                    self.picture_view.set_filename(Some(pic.path));
                    self.picture_navigation_view.push_by_tag("picture");
                } else {
                    println!("Failed loading {}: {:?}", picture_id, result);
                }
            }
        }
    }

    fn shutdown(&mut self, widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        widgets.save_window_size().unwrap();
    }
}

fn convert_all_photos_output(msg: PhotoGridOutput) -> AppMsg {
    match msg {
        PhotoGridOutput::ViewPhoto(id) => AppMsg::ViewPhoto(id),
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
