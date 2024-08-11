// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use i18n_embed::fluent::FluentLanguageLoader;

use crate::config::I18NDIR;
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::I18nEmbedError;
use i18n_embed::LanguageLoader;

use lazy_static::lazy_static;
use unic_langid::LanguageIdentifier;

use std::path::PathBuf;
use tracing::info;

lazy_static! {
    pub static ref LANGUAGE_LOADER: FluentLanguageLoader =
        loader().expect("i18n should be present");
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

pub fn loader() -> Result<FluentLanguageLoader, I18nEmbedError> {
    info!("I18NDIR = {:?}", I18NDIR);

    // Get user's preferred languages from OS.
    let requested_languages = DesktopLanguageRequester::requested_languages();
    info!("Requested languages: {:?}", requested_languages);

    // FIXME why can't all languages be derived from file system assets?
    // The 'available_languages() methods don't return all the languages :-/
    let all_languages = &[
        "de", "en-US", "fi", "fr", "hi", "id", "it", "nb-NO", "nl", "ru", "tr",
    ];

    let all_languages: Vec<LanguageIdentifier> =
        all_languages.iter().map(|id| id.parse().unwrap()).collect();

    let i18n_assets = i18n_embed::FileSystemAssets::try_new(PathBuf::from(I18NDIR))?;

    let loader = FluentLanguageLoader::new("fotema", "en-US".parse().unwrap());
    loader.load_languages(&i18n_assets, &all_languages)?;

    let loader = loader.select_languages_negotiate(
        &requested_languages,
        i18n_embed::fluent::NegotiationStrategy::Filtering,
    );

    // FIXME I don't understand why I have to create a second loader to get the correct
    // languages to be used :-/

    let negotiated_languates = loader.current_languages();
    let loader = FluentLanguageLoader::new("fotema", "en-US".parse().unwrap());
    loader.load_languages(&i18n_assets, &negotiated_languates)?;

    info!("Final languages: {:?}", loader.current_languages());

    Ok(loader)
}
