CREATE TABLE IF NOT EXISTS library_roots (
    root_id INTEGER PRIMARY KEY AUTOINCREMENT,
    host_path_b64 TEXT UNIQUE NOT NULL
);

-- Insert fallback default root for existing databases
INSERT INTO library_roots (root_id, host_path_b64) VALUES (1, '') ON CONFLICT DO NOTHING;

-- Recreate pictures table to change UNIQUE(picture_path_b64) to UNIQUE(root_id, picture_path_b64)
CREATE TABLE pictures_new (
    picture_id INTEGER PRIMARY KEY UNIQUE NOT NULL,
    root_id INTEGER NOT NULL DEFAULT 1 REFERENCES library_roots(root_id) ON DELETE CASCADE,
    picture_path_b64 TEXT NOT NULL,
    picture_path_lossy TEXT NOT NULL,
    thumbnail_path TEXT UNIQUE,
    fs_created_ts DATETIME,
    fs_modified_ts DATETIME,
    exif_created_ts DATETIME,
    exif_modified_ts DATETIME,
    is_selfie BOOLEAN CHECK (is_selfie IN (0, 1)),
    link_path_b64 TEXT NOT NULL,
    link_path_lossy TEXT NOT NULL,
    content_id TEXT,
    metadata_version INTEGER NOT NULL DEFAULT 0,
    orientation INTEGER,
    is_broken BOOLEAN DEFAULT 0,
    insert_ts DATETIME NOT NULL DEFAULT '1970-01-01 00:00:00',
    UNIQUE (root_id, picture_path_b64)
);

INSERT INTO pictures_new (
    picture_id, root_id, picture_path_b64, picture_path_lossy, thumbnail_path,
    fs_created_ts, fs_modified_ts, exif_created_ts, exif_modified_ts, is_selfie,
    link_path_b64, link_path_lossy, content_id, metadata_version, orientation,
    is_broken, insert_ts
)
SELECT
    picture_id, 1, picture_path_b64, picture_path_lossy, thumbnail_path,
    fs_created_ts, fs_modified_ts, exif_created_ts, exif_modified_ts, is_selfie,
    link_path_b64, link_path_lossy, content_id, metadata_version, orientation,
    is_broken, insert_ts
FROM pictures;

DROP VIEW IF EXISTS visual;
DROP VIEW IF EXISTS pictures_cleanup;
DROP TABLE pictures;
ALTER TABLE pictures_new RENAME TO pictures;

CREATE INDEX pic_live_photo_idx ON pictures(link_path_b64, content_id);
CREATE INDEX pic_root_idx ON pictures(root_id);

-- Recreate videos table to change UNIQUE(video_path_b64) to UNIQUE(root_id, video_path_b64)
CREATE TABLE videos_new (
    video_id INTEGER PRIMARY KEY UNIQUE NOT NULL,
    root_id INTEGER NOT NULL DEFAULT 1 REFERENCES library_roots(root_id) ON DELETE CASCADE,
    video_path_b64 TEXT NOT NULL,
    video_path_lossy TEXT NOT NULL,
    link_path_b64 TEXT NOT NULL,
    link_path_lossy TEXT NOT NULL,
    thumbnail_path TEXT UNIQUE,
    fs_created_ts DATETIME,
    fs_modified_ts DATETIME,
    stream_created_ts DATETIME,
    duration_millis INTEGER,
    video_codec TEXT,
    transcoded_path TEXT,
    content_id TEXT,
    metadata_version INTEGER NOT NULL DEFAULT 0,
    rotation INTEGER,
    is_broken BOOLEAN DEFAULT 0,
    insert_ts DATETIME NOT NULL DEFAULT '1970-01-01 00:00:00',
    UNIQUE (root_id, video_path_b64)
);

INSERT INTO videos_new (
    video_id, root_id, video_path_b64, video_path_lossy, link_path_b64, link_path_lossy,
    thumbnail_path, fs_created_ts, fs_modified_ts, stream_created_ts, duration_millis,
    video_codec, transcoded_path, content_id, metadata_version, rotation,
    is_broken, insert_ts
)
SELECT
    video_id, 1, video_path_b64, video_path_lossy, link_path_b64, link_path_lossy,
    thumbnail_path, fs_created_ts, fs_modified_ts, stream_created_ts, duration_millis,
    video_codec, transcoded_path, content_id, metadata_version, rotation,
    is_broken, insert_ts
FROM videos;

DROP VIEW IF EXISTS videos_cleanup;
DROP TABLE videos;
ALTER TABLE videos_new RENAME TO videos;

CREATE INDEX vid_live_photo_idx ON videos(link_path_b64, content_id);
CREATE INDEX vid_root_idx ON videos(root_id);

-- Recreate visual view with root_id
CREATE VIEW visual AS
SELECT
  -- Unique ID
  COALESCE(pictures.picture_id, 'x') || '_' || COALESCE(videos.video_id, 'x') AS visual_id,
  COALESCE(pictures.root_id, videos.root_id) AS root_id,
  COALESCE(pictures.link_path_b64, videos.link_path_b64) AS link_path_b64,

  pictures.picture_id,
  pictures.picture_path_b64,
  pictures.picture_path_lossy,
  pictures.orientation AS picture_orientation,
  pictures.is_selfie,

  videos.video_id,
  videos.video_path_b64,
  videos.video_path_lossy,

  COALESCE(videos.video_codec, motion_photos.video_codec) AS video_codec,
  false AS is_transcode_required,
  COALESCE(videos.transcoded_path, motion_photos.transcoded_path) AS video_transcoded_path,
  COALESCE(videos.rotation, motion_photos.rotation) AS video_rotation,

  CASE
        WHEN videos.content_id IS NOT NULL THEN true
        WHEN videos.duration_millis <= 3000 THEN true
        WHEN motion_photos.video_path IS NOT NULL THEN true
        ELSE false
  END AS is_live_photo,

  COALESCE(videos.duration_millis, motion_photos.duration_millis) as duration_millis,
  motion_photos.video_path AS motion_photo_video_path,

  pictures_geo.longitude AS longitude,
  pictures_geo.latitude AS latitude,

  COALESCE(
    pictures.exif_created_ts,
    videos.stream_created_ts,
    pictures.exif_modified_ts,
    pictures.fs_created_ts,
    videos.fs_created_ts,
    pictures.fs_modified_ts,
    videos.fs_modified_ts,
    pictures.insert_ts,
    videos.insert_ts,
    CURRENT_TIMESTAMP
  ) AS ordering_ts
FROM pictures
LEFT JOIN videos ON pictures.link_path_b64 = videos.link_path_b64 AND pictures.root_id = videos.root_id
LEFT JOIN motion_photos ON pictures.picture_id = motion_photos.picture_id
LEFT JOIN pictures_geo ON pictures.picture_id = pictures_geo.picture_id

UNION

SELECT
  COALESCE(pictures.picture_id, 'x') || '_' || COALESCE(videos.video_id, 'x') AS visual_id,
  COALESCE(pictures.root_id, videos.root_id) AS root_id,
  COALESCE(pictures.link_path_b64, videos.link_path_b64) AS link_path_b64,

  pictures.picture_id,
  pictures.picture_path_b64,
  pictures.picture_path_lossy,
  pictures.orientation AS picture_orientation,
  pictures.is_selfie,

  videos.video_id,
  videos.video_path_b64,
  videos.video_path_lossy,

  COALESCE(videos.video_codec, motion_photos.video_codec) AS video_codec,
  false AS is_transcode_required,
  COALESCE(videos.transcoded_path, motion_photos.transcoded_path) AS video_transcoded_path,
  COALESCE(videos.rotation, motion_photos.rotation) AS video_rotation,

  CASE
        WHEN videos.content_id IS NOT NULL THEN true
        WHEN videos.duration_millis <= 3000 THEN true
        WHEN motion_photos.video_path IS NOT NULL THEN true
        ELSE false
  END AS is_live_photo,

  COALESCE(videos.duration_millis, motion_photos.duration_millis) as duration_millis,
  motion_photos.video_path AS motion_photo_video_path,

  pictures_geo.longitude AS longitude,
  pictures_geo.latitude AS latitude,

  COALESCE(
    pictures.exif_created_ts,
    videos.stream_created_ts,
    pictures.exif_modified_ts,
    pictures.fs_created_ts,
    videos.fs_created_ts,
    pictures.fs_modified_ts,
    videos.fs_modified_ts,
    pictures.insert_ts,
    videos.insert_ts,
    CURRENT_TIMESTAMP
  ) AS ordering_ts
FROM videos
LEFT JOIN pictures ON videos.link_path_b64 = pictures.link_path_b64 AND videos.root_id = pictures.root_id
LEFT JOIN motion_photos ON pictures.picture_id = motion_photos.picture_id
LEFT JOIN pictures_geo ON pictures.picture_id = pictures_geo.picture_id;

-- Recreate pictures_cleanup and videos_cleanup views
CREATE VIEW pictures_cleanup AS
SELECT
        picture_id,
        'cache' AS root_name,
        'picture thumbnail' AS description,
        thumbnail_path AS path
FROM pictures

UNION

SELECT
        picture_id,
        'cache' AS root_name,
        'motion photo video' AS description,
        video_path AS path
FROM motion_photos
WHERE video_path IS NOT NULL

UNION

SELECT
        picture_id,
        'cache' AS root_name,
        'motion photo transcoded video' AS description,
        transcoded_path AS path
FROM motion_photos
WHERE transcoded_path IS NOT NULL

UNION

SELECT
        picture_id,
        'data' AS root_name,
        'face bounds' AS description,
        bounds_path AS path
FROM pictures_faces

UNION

SELECT
        picture_id,
        'data' AS root_name,
        'face thumbnail' AS description,
        thumbnail_path AS path
FROM pictures_faces;

CREATE VIEW videos_cleanup AS
SELECT
        video_id,
        'cache' AS root_name,
        'video thumbnail' AS description,
        thumbnail_path AS path
FROM videos

UNION

SELECT
        video_id,
        'cache' AS root_name,
        'transcoded video' AS description,
        transcoded_path AS path
FROM videos
WHERE transcoded_path IS NOT NULL;
