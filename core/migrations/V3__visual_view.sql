CREATE VIEW visual AS
SELECT
  -- Unique ID
  COALESCE(pictures.picture_id, 'x') || '_' || COALESCE(videos.video_id, 'x') AS visual_id,
  COALESCE(pictures.link_path, videos.link_path) as link_path,
  pictures.picture_id,
  pictures.picture_path,

-- If the thumbnail path is absent in the database, then compute the path we know it
-- will have. Eventually the thumbnail generation background process will create the file
-- and it will show up in the UI without having to refresh the data.
  CASE pictures.picture_id
        WHEN NOT NULL THEN pictures.thumbnail_path
        ELSE 'photo_thumbnails/' || printf('%04d', pictures.picture_id / 1000) || '/' || CAST(pictures.picture_id AS TEXT) || '_200x200.png'
  END AS picture_thumbnail,

  pictures.is_selfie,
  videos.video_id,
  videos.video_path,

-- If the thumbnail path is absent in the database, then compute the path we know it
-- will have. Eventually the thumbnail generation background process will create the file
-- and it will show up in the UI without having to refresh the data.
  CASE videos.video_id
        WHEN NOT NULL THEN videos.thumbnail_path
        ELSE 'video_thumbnails/' || printf('%04d', videos.video_id / 1000) || '/' || CAST(videos.video_id AS TEXT) || '_200x200.png'
  END AS video_thumbnail,

  videos.video_codec,
  videos.transcoded_path AS video_transcoded_path,
  videos.video_codec IN ('hevc') AS is_transcode_required,
  videos.content_id IS NOT NULL AS is_ios_live_photo,
  videos.duration_millis as duration_millis,
  COALESCE(
    pictures.exif_created_ts,
    videos.stream_created_ts,
    pictures.exif_modified_ts,
    pictures.fs_created_ts,
    videos.fs_created_ts
  ) AS created_ts
FROM
  pictures
  FULL OUTER JOIN videos USING (link_path, content_id)
ORDER BY
  created_ts ASC

