-- Tracks whether a picture's embedded XMP face-region person tags have been
-- imported, so the (file-reading) import runs at most once per picture.
ALTER TABLE pictures ADD COLUMN face_tags_imported INTEGER NOT NULL DEFAULT 0;
