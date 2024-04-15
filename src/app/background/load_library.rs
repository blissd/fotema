// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use relm4::prelude::*;
use relm4::Worker;

#[derive(Debug)]
pub enum LoadLibraryInput {
    Refresh,
}

#[derive(Debug)]
pub enum LoadLibraryOutput {
    Refreshed,
}

pub struct LoadLibrary {
    library: fotema_core::visual::Library,
}

impl Worker for LoadLibrary {
    type Init = fotema_core::visual::Library;
    type Input = LoadLibraryInput;
    type Output = LoadLibraryOutput;

    fn init(library: Self::Init, _sender: ComponentSender<Self>) -> Self {
        Self { library }
    }

    fn update(&mut self, msg: LoadLibraryInput, sender: ComponentSender<Self>) {
        match msg {
            LoadLibraryInput::Refresh => {
                let result = self.library.refresh();
                if let Ok(_) = result {
                    let _ = sender.output(LoadLibraryOutput::Refreshed);
                } else if let Err(e) = result {
                    println!("Failed scan with: {}", e);
                }
            }
        };
    }
}
