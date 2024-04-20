-- A photo in the library
CREATE TABLE pictures (
        picture_id       INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for picture
        picture_path     TEXT UNIQUE NOT NULL, -- path to picture
        thumbnail_path   TEXT UNIQUE, -- path to picture thumbnail
        fs_created_ts    DATETIME NOT NULL, -- UTC timestamp from file system
        exif_created_ts  DATETIME, -- UTC timestamp for EXIF original creation date
        exif_modified_ts DATETIME, -- UTC timestamp for EXIF original modification date
        is_selfie        BOOLEAN CHECK (is_selfie IN (0, 1)), -- front camera?
        link_path TEXT NOT NULL,
        link_date TEXT
);

CREATE INDEX  pic_link_path ON pictures(link_path);

