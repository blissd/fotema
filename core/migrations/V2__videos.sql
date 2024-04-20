-- A video in the library
CREATE TABLE videos (
        video_id          INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for video
        video_path        TEXT UNIQUE NOT NULL, -- path to video
        thumbnail_path    TEXT UNIQUE, -- path to preview
        fs_created_ts     DATETIME NOT NULL, -- UTC timestamp of file system creation time
        stream_created_ts DATETIME, -- UTC creation timestamp from video stream metadata
        duration_millis   INTEGER, -- Duration in milliseconds of video
        video_codec       TEXT, -- Video codec.
        link_path         TEXT NOT NULL, -- Parent path for linking with sibling photos
        link_date         TEXT -- creation date without time
);

CREATE INDEX  vid_link_path ON videos(link_path);

