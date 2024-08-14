-- Glycin is now used to apply EXIF rotation data to images before
-- generating thumbnails, which means Fotema no longer needs to use CSS
-- to orient thumbnails correctly in the various album views.
-- Consequently, all rotated thumbnails must be regenerated so they don't
-- get double rotated.
--
-- Note that between this SQL being applied and the thumbnails being updated,
-- there will be a period of time that the user will see incorrectly oriented
-- thumbnails.
--
-- Don't need to regenerate thumbnail if orientation is "North" (1) or NULL.

UPDATE pictures
SET thumbnail_path = NULL
WHERE
orientation IN (2, 3, 4, 5, 6, 7, 8)
;

