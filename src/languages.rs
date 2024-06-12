// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};

use crate::config::I18NDIR;
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::LanguageLoader;
use lazy_static::lazy_static;
use unic_langid::LanguageIdentifier;

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
    let loader: FluentLanguageLoader = fluent_language_loader!();
    let localizations = i18n_embed::FileSystemAssets::new(PathBuf::from(I18NDIR));

    // Get user's preferred languages from OS.
    let requested_languages = DesktopLanguageRequester::requested_languages();

    info!("Requested languages: {:?}", requested_languages);

    // For each requested language add a more general version _without_ a region specified.
    // This is to make language fallbacks work properly so that "de_DE" can fallback to "de"
    // rather than falling back to "en_US".
    // Apology to future self: I suspect I shouldn't have to do this, and that this solution
    // will come back to bite you in the bum.
    let mut requested_languages = requested_languages
        .into_iter()
        .flat_map(|lang| {
            vec![
                lang.clone(),
                LanguageIdentifier::from_parts(lang.language, None, None, &[]),
            ]
        })
        .collect::<Vec<_>>();

    if requested_languages.is_empty() {
        // FIXME why doesn't setting the fallback language on the FluentLanguageLoader
        // work when there isn't a requested language?
        let fallback: LanguageIdentifier = "en-US".parse().unwrap();
        requested_languages.push(fallback);
    }

    let requested_languages = &requested_languages.iter().collect::<Vec<_>>(); // janky API needs &[&lang_id]

    info!(
        "Requested languages with fallbacks: {:?}",
        requested_languages
    );

    loader
        .load_languages(&localizations, &requested_languages)
        .expect("Localization files should be present");
    loader
}
