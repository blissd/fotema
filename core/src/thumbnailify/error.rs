// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
use thiserror::Error;

/// A unified error type for the thumbnail library.
#[derive(Error, Debug)]
pub enum ThumbnailError {
    /// Wraps errors originating from the `image` crate.
    #[error("Image crate error: {0}")]
    Image(#[from] image::ImageError),

    /// Wraps standard I/O errors.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    //#[error("INI config error: {0}")]
    //Ini(#[from] ini::Error),
    #[error("File persistence error: {0}")]
    Persist(#[from] tempfile::PersistError),

    //#[error("Shell parse error: {0}")]
    //Parse(#[from] shell_words::ParseError),
    #[error("PNG encoding error: {0}")]
    PngEncoding(#[from] png::EncodingError),

    #[error("PNG decoding error: {0}")]
    PngDecoding(#[from] png::DecodingError),

    #[error("Image resize error: {0}")]
    ResizeError(#[from] fast_image_resize::ResizeError),
}
