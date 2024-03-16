<!--
SPDX-FileCopyrightText: © 2024 David Bliss

SPDX-License-Identifier: GPL-3.0-or-later
-->

# rust-lib-photos

Rust API for scanning photos on a local file system, extracting EXIF and file system metadata, and persisting
that data to an SQLite database.

## Example:

```rust
use photos::Controller;
use photos::Repository;
use photos::Scanner;
use std::path::PathBuf;
use tempfile;

fn scan_pictures() {
    let repo = {
        let mut db_path = tempfile::tempdir().unwrap().into_path();
        db_path.push("test.sqlite");
        Repository::open(&db_path).unwrap()
    };

    let scanner = {
        let pic_dir = PathBuf::from(env!("XDG_PICTURES_DIR"));
        Scanner::build(&pic_dir).unwrap()
    };

    let ctl = Controller::new(repo, scanner);

    ctl.scan().unwrap();

    let all_pics = ctl.all().unwrap();
    for pic in all_pics {
        println!("{:?}", pic.relative_path);
    }
}
```
