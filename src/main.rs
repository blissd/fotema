// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

#[rustfmt::skip]
mod app;
mod adaptive;
mod config;
mod languages;

use app::App;

use config::{APP_ID, GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};
use gettextrs::{LocaleCategory, gettext};
use gtk::prelude::ApplicationExt;
use gtk::{gio, glib};
use relm4::{
    RelmApp,
    actions::{AccelsPlus, RelmAction, RelmActionGroup},
    gtk, main_application,
};

use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

relm4::new_action_group!(AppActionGroup, "app");
relm4::new_stateless_action!(QuitAction, AppActionGroup, "quit");

fn main() {
    gtk::init().unwrap();

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::ERROR.into())
        .from_env_lossy(); // picks up RUST_LOG

    // Enable logging
    tracing_subscriber::fmt()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::FULL)
        .with_max_level(tracing::Level::INFO)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::new(
            "%T%.3f".into(),
        ))
        .with_env_filter(env_filter)
        .compact()
        .init();

    // setup gettext
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // OpenStreetMap likes clients to provide a user agent so API overuse or abuse can
    // be easily identified. Hopefully Fotema users won't cause OSM too much trouble, but
    // I'd like to be compliant with the OSM policies just in case they need to get in touch.
    // OSM--Thank you for your service.
    shumate::functions::set_user_agent(Some("Fotema Photo Gallery for Linux (https://fotema.app)"));

    glib::set_application_name(&gettext("Fotema"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);

    gtk::Window::set_default_icon_name(APP_ID);

    let app = main_application();
    app.set_resource_base_path(Some("/app/fotema/Fotema/"));
    app.set_application_id(Some(APP_ID));

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
            "/app/fotema/Fotema/style.css",
            gio::ResourceLookupFlags::NONE,
        )
        .unwrap();
    relm4::set_global_css(&glib::GString::from_utf8_checked(data.to_vec()).unwrap());
    app.visible_on_activate(false).run::<App>(());
}
