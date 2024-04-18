-- A photo in the library
CREATE TABLE pictures (
        picture_id       INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for picture
        picture_path     TEXT UNIQUE NOT NULL, -- path to picture
        preview_path     TEXT UNIQUE, -- path to picture preview
        fs_created_ts    DATETIME NOT NULL, -- UTC timestamp from file system
        exif_created_ts  DATETIME, -- UTC timestamp for EXIF original creation date
        exif_modified_ts DATETIME, -- UTC timestamp for EXIF original modification date
        is_selfie        BOOLEAN CHECK (is_selfie IN (0, 1)) -- front camera?
);

-- A video in the library
CREATE TABLE videos (
        video_id          INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for video
        video_path        TEXT UNIQUE NOT NULL, -- path to video
        preview_path      TEXT UNIQUE, -- path to preview
        fs_created_ts     DATETIME NOT NULL, -- UTC timestamp of file system creation time
        stream_created_ts DATETIME, -- UTC creation timestamp from video stream metadata
        duration_millis   INTEGER -- Duration in milliseconds of video
);

-- Visual artefacts. Either a photo, a video, or both at once.
CREATE TABLE visual (
        visual_id     INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for video
        picture_id    TEXT UNIQUE, -- path to video
        video_id      TEXT UNIQUE, -- path to preview
        stem_path     TEXT UNIQUE NOT NULL, -- visual artefact path minus suffix
        FOREIGN KEY (picture_id) REFERENCES pictures (picture_id) ON DELETE CASCADE,
        FOREIGN KEY (video_id)   REFERENCES videos   (video_id)   ON DELETE CASCADE,
        CONSTRAINT one_of_picture_or_video CHECK ((picture_id IS NOT NULL) OR (video_id IS NOT NULL))
)
