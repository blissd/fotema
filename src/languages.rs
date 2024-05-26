// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};

use crate::config::I18NDIR;
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::LanguageLoader;
use lazy_static::lazy_static;

use std::path::PathBuf;
use tracing::info;

lazy_static! {
    pub static ref LANGUAGE_LOADER: FluentLanguageLoader = loader();
}

// Wrap fl macro so the language loader doesn't have be specified on each call.
// See https://crates.io/crates/i18n-embed-fl
// Note to self: exports at crate level, use with "use crate::fl".
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        i18n_embed_fl::fl!($crate::languages::LANGUAGE_LOADER, $message_id)
    }};

    ($message_id:literal, $($args:expr),*) => {{
        i18n_embed_fl::fl!($crate::languages::LANGUAGE_LOADER, $message_id, $($args), *)
    }};
}

pub fn loader() -> FluentLanguageLoader {
    // Get user's preferred languages from OS.
    let requested_languages = DesktopLanguageRequester::requested_languages();
    let requested_languages = &requested_languages.iter().collect::<Vec<_>>(); // janky API needs &[&lang_id]

    info!("Requested languages: {:?}", requested_languages);

    let loader: FluentLanguageLoader = fluent_language_loader!();
    let i18n_assets = i18n_embed::FileSystemAssets::new(PathBuf::from(I18NDIR));

    loader
        .load_languages(&i18n_assets, &requested_languages)
        .expect("Localization files should be present");
    loader
}
