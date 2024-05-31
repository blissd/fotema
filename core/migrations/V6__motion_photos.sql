
-- Motion photos extracted from photo files
CREATE TABLE motion_photos (
        -- unique ID for picture
        picture_id         INTEGER PRIMARY KEY UNIQUE NOT NULL,

        -- version number of video extraction code. Used for easy reprocessing
        -- when extraction logic changes.
        extract_version    INTEGER NOT NULL DEFAULT 0,

        -- path to extracted video under cache directory
        video_path         TEXT UNIQUE,

        -- path to transcoded video under cache directory
        transcoded_path    TEXT UNIQUE,

        -- video duration in milliseconds
        duration_millis    INTEGER,

        -- video codec, such as HEVC
        video_codec        TEXT,

        -- rotation in degrees
        rotation           INTEGER,

        FOREIGN KEY (picture_id) REFERENCES pictures (picture_id) ON DELETE CASCADE
);

