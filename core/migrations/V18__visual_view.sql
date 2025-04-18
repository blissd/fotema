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
  pictures.is_selfie,

  videos.video_id,
  videos.video_path_b64,
  videos.video_path_lossy, -- for debug only. Never read in Fotema.

  COALESCE(videos.video_codec, motion_photos.video_codec) AS video_codec,

  -- GNOME 48 runtime appears to support HEVC videos without transcoding.
  false AS is_transcode_required,

  COALESCE(videos.transcoded_path, motion_photos.transcoded_path) AS video_transcoded_path,

  COALESCE(videos.rotation, motion_photos.rotation) AS video_rotation,

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

  pictures_geo.longitude AS longitude,
  pictures_geo.latitude AS latitude,

  -- Timestamp to order visual items by.
  -- Prefer embedded metadata over file system metadata.
  COALESCE(
    pictures.exif_created_ts,
    videos.stream_created_ts,
    pictures.exif_modified_ts,
    pictures.fs_created_ts,
    videos.fs_created_ts,
    pictures.fs_modified_ts,
    videos.fs_modified_ts,
    CURRENT_TIMESTAMP
  ) AS ordering_ts
FROM
  pictures
  FULL OUTER JOIN videos USING (link_path_b64, content_id)
  FULL OUTER JOIN motion_photos USING (picture_id)
  FULL OUTER JOIN pictures_geo USING (picture_id)
WHERE COALESCE(pictures.is_broken, FALSE) IS FALSE
AND COALESCE(videos.is_broken, FALSE) IS FALSE
ORDER BY
  ordering_ts ASC;

