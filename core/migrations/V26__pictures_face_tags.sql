-- Person-name face regions read from a photo's XMP (MWG / Microsoft people tags),
-- cached so the recognition pass can match them to detected faces without
-- re-opening the photo file (they are read once, together with EXIF, in enrich).
CREATE TABLE pictures_face_tags (
    picture_id INTEGER NOT NULL REFERENCES pictures (picture_id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    center_x   REAL NOT NULL
);

CREATE INDEX ix_pictures_face_tags_picture_id ON pictures_face_tags (picture_id);
