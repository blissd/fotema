CREATE VIEW visual AS
SELECT
  -- Unique ID
  COALESCE(pictures.picture_id, 'x') || '_' || COALESCE(videos.video_id, 'x') AS visual_id,
  COALESCE(pictures.link_path, videos.link_path) as link_path,
  pictures.picture_id,
  pictures.picture_path,
  pictures.thumbnail_path AS picture_thumbnail,
  videos.video_id,
  videos.video_path,
  videos.thumbnail_path AS video_thumbnail,
  videos.video_codec,
  videos.transcoded_path AS video_transcoded_path,
  videos.video_codec IN ('hevc') AS is_transcode_required,
  videos.content_id IS NOT NULL AS is_ios_live_photo,
  COALESCE(
    pictures.exif_created_ts,
    pictures.exif_modified_ts,
    videos.stream_created_ts,
    pictures.fs_created_ts,
    videos.fs_created_ts
  ) AS created_ts
FROM
  pictures
  FULL OUTER JOIN videos USING (link_path, content_id)
WHERE
  COALESCE(pictures.thumbnail_path, videos.thumbnail_path) IS NOT NULL
ORDER BY
  created_ts ASC




