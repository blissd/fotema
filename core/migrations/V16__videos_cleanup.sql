-- A view of all cache and data files created for videos and that should
-- be deleted if the video in no longer present.

CREATE VIEW videos_cleanup AS

SELECT video_id, 'cache' AS root_name, 'video thumbnail' AS description, thumbnail_path AS path
FROM videos

UNION

SELECT video_id, 'cache' AS root_name, 'video transcode' AS description, transcoded_path AS path
FROM videos
WHERE transcoded_path IS NOT NULL

