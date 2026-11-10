// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

const PICTURES_SUFFIXES: [&str; 11] = [
    "avif", "exr", "heic", "jpeg", "jpg", "jxl", "png", "qoi", "tiff", "webp", "gif",
];

// Broad set of video container extensions that FFmpeg (used by the enrich,
// thumbnail and transcode steps) can typically handle. Recognising a file here
// only adds it to the scan; if FFmpeg can't actually decode it, the existing
// broken-media handling marks it as broken rather than crashing.
const VIDEO_SUFFIXES: &[&str] = &[
    "mp4", "m4v", "mov", "qt", "avi", "divx", "mkv", "webm", "3gp", "3g2", "3gpp", "mts", "m2ts",
    "ts", "mpg", "mpeg", "mpe", "m2v", "mpv", "m2p", "wmv", "asf", "flv", "f4v", "vob", "ogv",
    "ogm", "mxf", "dv", "rm", "rmvb", "mod", "tod",
];

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

    for vid_ext in VIDEO_SUFFIXES.iter().copied() {
        if path_ext.eq_ignore_ascii_case(vid_ext) {
            return true;
        }
    }

    return false;
}
