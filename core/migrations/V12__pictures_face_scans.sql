
-- Face detection runs
CREATE TABLE pictures_face_scans (
        picture_id   INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for picture
        is_broken    BOOLEAN NOT NULL CHECK (is_broken IN (0, 1)) DEFAULT 1, -- scan failed?
        scan_ts      DATETIME NOT NULL, -- UTC timestamp of scan
        face_count   INTEGER NOT NULL, -- count of faces found

        FOREIGN KEY (picture_id) REFERENCES pictures (picture_id) ON DELETE CASCADE
);

