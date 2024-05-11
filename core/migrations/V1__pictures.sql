-- A photo in the library
CREATE TABLE pictures (
        picture_id         INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for picture
        picture_path_b64   TEXT UNIQUE NOT NULL, -- path to picture (base64 encoded)
        picture_path_lossy TEXT NOT NULL, --path to picture. Human readable for debugging.
        thumbnail_path     TEXT UNIQUE, -- path to picture thumbnail. Not b64 as we only build UTF8 paths.
        fs_created_ts      DATETIME NOT NULL, -- UTC timestamp from file system
        exif_created_ts    DATETIME, -- UTC timestamp for EXIF original creation date
        exif_modified_ts   DATETIME, -- UTC timestamp for EXIF original modification date
        is_selfie          BOOLEAN CHECK (is_selfie IN (0, 1)), -- front camera?
        link_path_b64      TEXT NOT NULL, -- picture parent path, for linking picture/photo siblings. Base64 encoded.
        link_path_lossy    TEXT NOT NULL, --picture parent path. Human readable for debugging.
        content_id         TEXT,
        metadata_version   INTEGER NOT NULL DEFAULT 0 -- code version that scanned metadata
);

CREATE INDEX  pic_live_photo_idx ON pictures(link_path_b64, content_id);

