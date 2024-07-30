-- Faces detected in pictures
CREATE TABLE pictures_faces (
        face_id        INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for face

        model_name     TEXT NOT NULL, -- face detection model used

        picture_id     INTEGER NOT NULL, -- unique ID for picture

        is_confirmed   BOOLEAN NOT NULL CHECK (is_confirmed IN (0, 1)) DEFAULT 0, -- person_id confirmed by user?

        detected_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP, -- timestamp when face was detected

        person_id      INTEGER, -- person associated with face

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
        FOREIGN KEY (person_id) REFERENCES people (person_id) ON DELETE SET NULL
);

