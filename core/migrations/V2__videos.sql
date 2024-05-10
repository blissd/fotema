-- A video in the library
CREATE TABLE videos (
        video_id          INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for video
        video_path_b64    TEXT UNIQUE NOT NULL, -- base64 encoded path to video
        video_path_lossy  TEXT NOT NULL, -- human readable path to video for debugging
        link_path_b64     TEXT NOT NULL, -- base64 encoded video path minus suffix for linking with sibling photos
        link_path_lossy   TEXT NOT NULL, -- human readable link path for debugging
        thumbnail_path    TEXT UNIQUE, -- path to thumbnail. Not b64 as we only build UTF8 paths.
        fs_created_ts     DATETIME NOT NULL, -- UTC timestamp of file system creation time
        stream_created_ts DATETIME, -- UTC creation timestamp from video stream metadata
        duration_millis   INTEGER, -- Duration in milliseconds of video
        video_codec       TEXT, -- Video codec.
        transcoded_path   TEXT, -- path to transcoded video. Not b64 as we only build UTF8 paths.
        content_id        TEXT, -- iOS ID for linking with sibling photos
        metadata_version  INTEGER NOT NULL DEFAULT 0 -- code version that scanned metadata
);

CREATE INDEX  vid_live_photo_idx ON videos(link_path_b64, content_id);

