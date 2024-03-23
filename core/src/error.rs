// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

/// Bespoke errors
#[derive(Debug)]
pub enum Error {
    RepositoryError(String),
    ScannerError(String),
    PreviewError(String),
}
