use photos::Repository;
use photos::Scanner;
use std::path::PathBuf;
use tempfile;

fn picture_dir() -> PathBuf {
    let mut test_data_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    test_data_dir.push("resources/test");
    test_data_dir
}

#[test]
fn test_scan_and_persist() {
    let mut db_path = tempfile::tempdir().unwrap().into_path();
    db_path.push("test.sqlite");

    let repo = Repository::build(&db_path).unwrap();

    // let pic_dir = picture_dir();
    let pic_dir = PathBuf::from("/var/home/david/Pictures");
    let scanner = Scanner::build(&pic_dir).unwrap();

    scanner.visit_all(|pic| {
        let pic = photos::repo::Picture {
            path: pic.path,
            fs_modified_at: pic.fs.as_ref().and_then(|x| x.modified_at),
            created_at: pic.exif.as_ref().and_then(|x| x.created_at),
            modified_at: pic.exif.as_ref().and_then(|x| x.modified_at),
            description: pic.exif.as_ref().and_then(|x| x.description.clone()),
        };
        repo.add(&pic).ok();
    });

    let all_pics = repo.all().unwrap();
    for pic in all_pics {
        println!("{:?}", pic.path);
    }
}
