// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum ScannedFile {
    Photo(PathBuf),
    Video(PathBuf),
}
