// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};

use crate::LOCALEDIR;
use i18n_embed::DesktopLanguageRequester;
use i18n_embed::LanguageLoader;

use std::fs;
use std::io;
use std::path::PathBuf;

use unic_langid::{LanguageIdentifier, LanguageIdentifierError};

pub fn loader() -> FluentLanguageLoader {
    println!("lang = {:?}", std::env::var("LANG"));

    let i18n_assets = i18n_embed::FileSystemAssets::new(PathBuf::from(LOCALEDIR));

    let loader: FluentLanguageLoader = fluent_language_loader!();

    let lang_ids = get_available_locales().unwrap();
    println!("Available locales = {:?}", lang_ids);
    let lang_ids = &lang_ids.iter().collect::<Vec<_>>(); // janky API needs &[&lang_id]
    loader
        .load_languages(&i18n_assets, lang_ids)
        .expect("don't die");

    println!("current_lang = {:?}", loader.current_languages());

    // Get user's preferred languages from OS.
    let requested_languages = DesktopLanguageRequester::requested_languages();
    println!("Requested languages: {:?}", requested_languages);

    loader.select_languages_negotiate(
        &requested_languages,
        i18n_embed::fluent::NegotiationStrategy::Filtering,
    );
    //i18n_embed::select(&loader, &i18n_assets, &requested_languages).expect("don't die again");

    loader
}

/// This helper function allows us to read the list
/// of available locales by reading the list of
/// directories in `./examples/resources`.
///
/// It is expected that every directory inside it
/// has a name that is a valid BCP47 language tag.
pub fn get_available_locales() -> Result<Vec<LanguageIdentifier>, io::Error> {
    let mut locales = vec![];

    let res_path = PathBuf::from(LOCALEDIR);
    let res_dir = fs::read_dir(res_path)?;
    for entry in res_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name() {
                if let Some(name) = name.to_str() {
                    let result: Result<LanguageIdentifier, LanguageIdentifierError> = name.parse();
                    if let Ok(langid) = result {
                        locales.push(langid);
                    } else {
                        println!("Failed parsing '{}': {:?}", name, result);
                    }
                }
            }
        }
    }
    Ok(locales)
}
