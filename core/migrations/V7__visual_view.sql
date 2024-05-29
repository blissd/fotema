DROP VIEW visual;

CREATE VIEW visual AS
SELECT
  -- Unique ID
  COALESCE(pictures.picture_id, 'x') || '_' || COALESCE(videos.video_id, 'x') AS visual_id,
  COALESCE(pictures.link_path_b64, videos.link_path_b64) AS link_path_b64,

  pictures.picture_id,
  pictures.picture_path_b64,
  pictures.picture_path_lossy, -- for debug only. Never read in Fotema.
  pictures.orientation AS picture_orientation,

-- If the thumbnail path is absent in the database, then compute the path we know it
-- will have. Eventually the thumbnail generation background process will create the file
-- and it will show up in the UI without having to refresh the data.
  CASE pictures.picture_id
        WHEN NOT NULL THEN pictures.thumbnail_path
        ELSE 'photo_thumbnails/' || printf('%04d', pictures.picture_id / 1000) || '/' || CAST(pictures.picture_id AS TEXT) || '_200x200.png'
  END AS picture_thumbnail,

  pictures.is_selfie,

  videos.video_id,
  videos.video_path_b64,
  videos.video_path_lossy, -- for debug only. Never read in Fotema.

-- If the thumbnail path is absent in the database, then compute the path we know it
-- will have. Eventually the thumbnail generation background process will create the file
-- and it will show up in the UI without having to refresh the data.
  CASE videos.video_id
        WHEN NOT NULL THEN videos.thumbnail_path
        ELSE 'video_thumbnails/' || printf('%04d', videos.video_id / 1000) || '/' || CAST(videos.video_id AS TEXT) || '_200x200.png'
  END AS video_thumbnail,

  videos.video_codec,
  videos.video_codec IN ('hevc') AS is_transcode_required,
  videos.transcoded_path AS video_transcoded_path,
  videos.rotation AS video_rotation,

  -- An iOS live photo is a photo and a video linked with a content ID.
  -- However, we only really need the video part, and short (<3 seconds)
  -- videos are possibly live photos that have a missing or misnamed photo.
  CASE
        WHEN videos.content_id IS NOT NULL THEN true
        WHEN videos.duration_millis <= 3000 THEN true
        WHEN motion_photos.video_path IS NOT NULL THEN true
        ELSE false
  END AS is_live_photo,

  COALESCE(videos.duration_millis, motion_photos.duration_millis) as duration_millis,

  motion_photos.video_path AS motion_photo_video_path,

  motion_photos.video_codec AS motion_photo_video_codec,

  -- Prefer embedded metadata over file system metadata
  COALESCE(
    pictures.exif_created_ts,
    videos.stream_created_ts,
    pictures.exif_modified_ts,
    pictures.fs_created_ts,
    videos.fs_created_ts
  ) AS created_ts
FROM
  pictures
  FULL OUTER JOIN videos USING (link_path_b64, content_id)
  FULL OUTER JOIN motion_photos USING (picture_id)
WHERE COALESCE(pictures.is_broken, FALSE) IS FALSE
AND COALESCE(videos.is_broken, FALSE) IS FALSE
ORDER BY
  created_ts ASC;

