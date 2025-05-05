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
FROM pictures_faces
;

