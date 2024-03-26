// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

#[rustfmt::skip]
mod config;
mod all_photos;
mod app;
mod modals;
mod month_photos;
mod year_photos;

use app::App;
use config::{APP_ID, GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};
use gettextrs::{gettext, LocaleCategory};
use gtk::prelude::ApplicationExt;
use gtk::{gio, glib};
use relm4::{
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    gtk, main_application, RelmApp,
};

relm4::new_action_group!(AppActionGroup, "app");
relm4::new_stateless_action!(QuitAction, AppActionGroup, "quit");

fn main() {
    gtk::init().unwrap();

    // Enable logging
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(tracing::Level::INFO)
        .init();

    // setup gettext
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    glib::set_application_name(&gettext("Photo Romantic"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    gtk::Window::set_default_icon_name(APP_ID);

    let app = main_application();
    app.set_resource_base_path(Some("/dev/romantics/Photos/"));

    let mut actions = RelmActionGroup::<AppActionGroup>::new();

    let quit_action = {
        let app = app.clone();
        RelmAction::<QuitAction>::new_stateless(move |_| {
            app.quit();
        })
    };
    actions.add_action(quit_action);
    actions.register_for_main_application();

    app.set_accelerators_for_action::<QuitAction>(&["<Control>q"]);

    let app = RelmApp::from_app(app);

    let data = res
        .lookup_data(
            "/dev/romantics/Photos/style.css",
            gio::ResourceLookupFlags::NONE,
        )
        .unwrap();
    app.set_global_css(&glib::GString::from_utf8_checked(data.to_vec()).unwrap());
    app.visible_on_activate(false).run::<App>(());
}
