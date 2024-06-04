-- File system creation time should be nullable as not all file systems
-- support it. Looking at you, EXT4.
-- Add file system modification time. Looks like Linux file systems should
-- have a modification time of file, but allow nullable just in case.

DROP VIEW visual;

ALTER TABLE pictures RENAME COLUMN fs_created_ts TO fs_created_ts_old;
ALTER TABLE pictures ADD COLUMN fs_created_ts DATETIME;
ALTER TABLE pictures ADD COLUMN fs_modified_ts DATETIME;
UPDATE pictures SET fs_created_ts = fs_created_ts_old;
ALTER TABLE pictures DROP COLUMN fs_created_ts_old;

ALTER TABLE videos RENAME COLUMN fs_created_ts TO fs_created_ts_old;
ALTER TABLE videos ADD COLUMN fs_created_ts DATETIME;
ALTER TABLE videos ADD COLUMN fs_modified_ts DATETIME;
UPDATE videos SET fs_created_ts = fs_created_ts_old;
ALTER TABLE videos DROP COLUMN fs_created_ts_old;

