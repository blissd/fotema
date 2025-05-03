-- Fotema 2.0 runs face detection on thumbnails, not the original source
-- images, which produces much smaller images for both the bounds_path
-- and thumbnail_path.

-- Confirmed faces
CREATE TABLE migrate_faces (
        face_id    INTEGER PRIMARY KEY NOT NULL,
        picture_id INTEGER NOT NULL,
        face_index INTEGER NOT NULL, -- unique ID within picture

        FOREIGN KEY (face_id) REFERENCES pictures_faces (face_id) ON DELETE CASCADE,
        FOREIGN KEY (picture_id) REFERENCES pictures (picture_id) ON DELETE CASCADE
);

-- Copy over faces where:
-- 1. Face is confirmed.
-- 2. Face is unconfirmed, but for a picture that has a confirmed face.
INSERT INTO migrate_faces (face_id, picture_id, face_index)
SELECT
        face_id,
        picture_id,
        RANK() OVER (PARTITION BY picture_id ORDER BY face_id ASC) as face_index
FROM pictures_faces
WHERE is_confirmed IS TRUE
OR (picture_id IN (SELECT DISTINCT picture_id FROM pictures_faces WHERE is_confirmed IS TRUE));

-- Delete faces for pictures that _don't_ contain a confirmed face (a person).
DELETE FROM pictures_faces
WHERE picture_id NOT IN (SELECT DISTINCT picture_id FROM migrate_faces);

-- This isn't the full migration: the rest will be completed in the Rust
-- code at core/src/people/migrate.rs
-- For each row in migrate_faces:
-- 1. Copy bounds_path image to new path.
-- 2. Copy thumbnail_path image to new path.
-- 3. Update paths in pictures_faces
--
-- Rust migration can then:
-- 1. Copy both images for each face to new paths.
-- 2. Update paths in pictures_faces table.
-- 3. Delete photo_faces directory to reclaim disk space.

