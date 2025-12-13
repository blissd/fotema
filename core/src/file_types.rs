// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

const PICTURES_SUFFIXES: [&str; 11] = [
    "avif", "exr", "heic", "jpeg", "jpg", "jxl", "png", "qoi", "tiff", "webp", "gif",
];

const VIDEO_SUFFIXES: [&str; 5] = ["m4v", "mov", "mp4", "avi", "mkv"];

pub fn is_supported_picture(path: &Path) -> bool {
    let Some(path_ext) = path.extension() else {
        return false;
    };

    for pic_ext in PICTURES_SUFFIXES {
        if path_ext.eq_ignore_ascii_case(pic_ext) {
            return true;
        }
    }

    return false;
}

pub fn is_supported_video(path: &Path) -> bool {
    let Some(path_ext) = path.extension() else {
        return false;
    };

    for pic_ext in VIDEO_SUFFIXES {
        if path_ext.eq_ignore_ascii_case(pic_ext) {
            return true;
        }
    }

    return false;
}
