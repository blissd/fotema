// SPDX-FileCopyrightText: © 2026 Benedikt Nimmervoll
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::{Path, PathBuf};

use fotema_core::thumbnailify::{
    ThumbnailSize, find_existing_thumbnail, get_thumbnail_hash_output,
    migrate_legacy_png_thumbnails,
};

fn create_thumbnail(dir: &Path, hash: &str, extension: &str) -> PathBuf {
    let size_dir = dir.join(ThumbnailSize::Small.to_string());
    fs::create_dir_all(&size_dir).unwrap();
    let path = size_dir.join(format!("{hash}.{extension}"));
    fs::write(&path, b"thumbnail").unwrap();
    path
}

#[test]
fn new_thumbnail_path_is_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let path = get_thumbnail_hash_output(dir.path(), "abc", ThumbnailSize::Small);
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("jpg"));
}

#[test]
fn finds_legacy_png_thumbnail() {
    let dir = tempfile::tempdir().unwrap();
    let png = create_thumbnail(dir.path(), "abc", "png");
    assert_eq!(
        find_existing_thumbnail(dir.path(), "abc", ThumbnailSize::Small),
        Some(png)
    );
}

#[test]
fn prefers_jpeg_over_legacy_png() {
    let dir = tempfile::tempdir().unwrap();
    create_thumbnail(dir.path(), "abc", "png");
    let jpg = create_thumbnail(dir.path(), "abc", "jpg");
    assert_eq!(
        find_existing_thumbnail(dir.path(), "abc", ThumbnailSize::Small),
        Some(jpg)
    );
}

#[test]
fn converts_legacy_png_thumbnail_to_jpeg() {
    let dir = tempfile::tempdir().unwrap();
    let size_dir = dir.path().join(ThumbnailSize::Small.to_string());
    fs::create_dir_all(&size_dir).unwrap();
    let png = size_dir.join("abc.png");
    image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]))
        .save_with_format(&png, image::ImageFormat::Png)
        .unwrap();

    let stats = migrate_legacy_png_thumbnails(dir.path());
    assert_eq!(stats.converted, 1);
    assert!(!png.exists());
    let jpg = size_dir.join("abc.jpg");
    assert!(jpg.exists());
    assert_eq!(
        find_existing_thumbnail(dir.path(), "abc", ThumbnailSize::Small),
        Some(jpg)
    );
}
