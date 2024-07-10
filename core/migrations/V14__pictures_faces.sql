-- Faces detected in pictures
CREATE TABLE pictures_faces (
        face_id        INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for face

        picture_id     INTEGER NOT NULL, -- unique ID for picture

        person_id      INTEGER, -- person associated with face

        thumbnail_path TEXT UNIQUE NOT NULL, -- path to square face thumbnail
        bounds_path    TEXT UNIQUE NOT NULL, -- path to face cropped to exact detected bounds

        bounds_x       INTEGER NOT NULL, -- face bounds X coordinate
        bounds_y       INTEGER NOT NULL, -- face bounds Y coordinate
        bounds_width   INTEGER NOT NULL, -- face bounds width
        bounds_height  INTEGER NOT NULL, -- face bounds height

        right_eye_x INTEGER, -- facial landmarks
        right_eye_y INTEGER,

        left_eye_x INTEGER,
        left_eye_y INTEGER,

        nose_x INTEGER,
        nose_y INTEGER,

        right_mouth_corner_x INTEGER,
        right_mouth_corner_y INTEGER,

        left_mouth_corner_x INTEGER,
        left_mouth_corner_y INTEGER,

        confidence DECIMAL NOT NULL, -- confidence (0.0 to 1.0) that detected face is a face.

        is_face    BOOLEAN NOT NULL CHECK (is_face IN (0, 1)) DEFAULT 1, -- is an actual face?

        FOREIGN KEY (picture_id) REFERENCES pictures (picture_id) ON DELETE CASCADE,
        FOREIGN KEY (person_id) REFERENCES people (person_id) ON DELETE SET NULL
);

