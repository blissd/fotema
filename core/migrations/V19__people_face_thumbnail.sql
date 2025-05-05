-- Remove people.thumbnail_path and replace with pictures_faces.is_thumbnail.
-- This is to make migration of people easier, which will happen in Fotema 2.0.

-- Must drop because view references pictures_faces, which is about to be
-- dropped and recreated.
DROP VIEW pictures_cleanup;

CREATE TABLE people2 (
        person_id         INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for person
        name              TEXT NOT NULL, -- name of person
        recognized_at     DATETIME NOT NULL DEFAULT '1970-01-01 00:00:00' -- timestamp of last face recognition scan
        -- default value is a date before Fotema was created and therefore before any face recognition runs ;-)
);

INSERT INTO people2 (person_id, name, recognized_at)
SELECT person_id, name, recognized_at FROM people;

-- Faces detected in pictures
CREATE TABLE pictures_faces2 (
        face_id        INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for face
        picture_id     INTEGER NOT NULL, -- unique ID for picture
        detected_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, -- timestamp when face was detected
        model_name     TEXT NOT NULL, -- face detection model used

        person_id      INTEGER, -- person associated with face
        is_thumbnail   BOOLEAN NOT NULL CHECK (is_confirmed IN (0, 1)) DEFAULT 0, -- is face also thumbnail for person
        is_confirmed   BOOLEAN NOT NULL CHECK (is_confirmed IN (0, 1)) DEFAULT 0, -- person_id confirmed by user?

        thumbnail_path TEXT UNIQUE NOT NULL, -- path to square face thumbnail
        bounds_path    TEXT UNIQUE NOT NULL, -- path to face cropped to exact detected bounds

        bounds_x       DECIMAL NOT NULL, -- face bounds X coordinate
        bounds_y       DECIMAL NOT NULL, -- face bounds Y coordinate
        bounds_width   DECIMAL NOT NULL, -- face bounds width
        bounds_height  DECIMAL NOT NULL, -- face bounds height

        right_eye_x DECIMAL NOT NULL, -- facial landmarks
        right_eye_y DECIMAL NOT NULL,

        left_eye_x DECIMAL NOT NULL,
        left_eye_y DECIMAL NOT NULL,

        nose_x DECIMAL NOT NULL,
        nose_y DECIMAL NOT NULL,

        right_mouth_corner_x DECIMAL NOT NULL,
        right_mouth_corner_y DECIMAL NOT NULL,

        left_mouth_corner_x DECIMAL NOT NULL,
        left_mouth_corner_y DECIMAL NOT NULL,

        confidence DECIMAL NOT NULL, -- confidence (0.0 to 1.0) that detected face is a face.

        is_ignored BOOLEAN NOT NULL CHECK (is_ignored IN (0, 1)) DEFAULT 0, -- ignored by user?

        FOREIGN KEY (picture_id) REFERENCES pictures (picture_id) ON DELETE CASCADE,
        FOREIGN KEY (person_id) REFERENCES people2 (person_id) ON DELETE SET NULL
);

INSERT INTO pictures_faces2
SELECT
        face_id,
        picture_id,
        detected_at,
        model_name,

        person_id,
        0 AS is_thumbnail,
        is_confirmed,

        thumbnail_path,
        bounds_path,

        bounds_x,
        bounds_y,
        bounds_width,
        bounds_height,

        right_eye_x,
        right_eye_y,

        left_eye_x,
        left_eye_y,

        nose_x,
        nose_y,

        right_mouth_corner_x,
        right_mouth_corner_y,

        left_mouth_corner_x,
        left_mouth_corner_y,

        confidence,

        is_ignored
FROM pictures_faces;

-- Use thumbnail paths in original people table to set is_thumbnail
-- on new pictures_faces table
UPDATE pictures_faces2 AS f
SET is_thumbnail = (f.thumbnail_path = p.thumbnail_path)
FROM people AS p
WHERE f.person_id = p.person_id;

DROP TABLE pictures_faces;
ALTER TABLE pictures_faces2 RENAME TO pictures_faces;

DROP TABLE people;
ALTER TABLE people2 RENAME TO people;

