// SPDX-FileCopyrightText: © 2026 Benedikt Nimmervoll
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::path::Path;

use fotema_core::file_types::{is_supported_picture, is_supported_video};

#[test]
fn recognises_common_extra_video_containers() {
    for name in [
        "clip.webm",
        "camera.mts",
        "camera.m2ts",
        "old.mpg",
        "phone.3gp",
        "movie.wmv",
        "cam.MP4",
    ] {
        assert!(is_supported_video(Path::new(name)), "{name}");
        assert!(!is_supported_picture(Path::new(name)), "{name}");
    }
}

#[test]
fn pictures_are_still_pictures() {
    for name in ["photo.jpg", "photo.heic", "photo.avif"] {
        assert!(is_supported_picture(Path::new(name)), "{name}");
        assert!(!is_supported_video(Path::new(name)), "{name}");
    }
}
