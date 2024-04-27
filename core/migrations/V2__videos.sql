-- A video in the library
CREATE TABLE videos (
        video_id          INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for video
        video_path        TEXT UNIQUE NOT NULL, -- path to video
        link_path         TEXT NOT NULL, -- video path minus suffix for linking with sibling photos
        thumbnail_path    TEXT UNIQUE, -- path to preview
        fs_created_ts     DATETIME NOT NULL, -- UTC timestamp of file system creation time
        stream_created_ts DATETIME, -- UTC creation timestamp from video stream metadata
        duration_millis   INTEGER, -- Duration in milliseconds of video
        video_codec       TEXT, -- Video codec.
        transcoded_path   TEXT, -- path to transcoded video
        content_id        TEXT, -- iOS ID for linking with sibling photos
        metadata_version  INTEGER NOT NULL DEFAULT 0 -- code version that scanned metadata
);

CREATE INDEX  vid_live_photo_idx ON videos(link_path, content_id);

