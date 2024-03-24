// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use photos_core::Controller;
use photos_core::Previewer;
use photos_core::Repository;
use photos_core::Scanner;
use std::path::PathBuf;
use tempfile;

fn picture_dir() -> PathBuf {
    let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_data_dir.push("resources/test");
    test_data_dir
}

#[test]
fn test_scan_and_persist() {
    //let pic_dir_base = picture_dir();
    let pic_dir_base = PathBuf::from("/var/home/david/Pictures");
    let repo = {
        let mut db_path = tempfile::tempdir().unwrap().into_path();
        db_path.push("test.sqlite");
        Repository::open(&pic_dir_base, &db_path).unwrap()
    };

    let scan = Scanner::build(&pic_dir_base).unwrap();

    let target_dir = PathBuf::from("target");
    let prev = Previewer::build(&target_dir).unwrap();

    let mut ctl = Controller::new(scan, repo, prev);

    ctl.scan().unwrap();

    let all_pics = ctl.all().unwrap();
    for pic in all_pics {
        println!("{:?}", pic.path);
    }
}
