

-- People named by the user who will faces associated with them
CREATE TABLE people (
        person_id         INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for person
        thumbnail_path    TEXT UNIQUE NOT NULL, -- path to primary person thumbnail
        name              TEXT NOT NULL, -- name of person
        recognized_at     DATETIME NOT NULL DEFAULT '1970-01-01 00:00:00' -- timestamp of last face recognition scan
                -- default value is a date before Fotema was created and therefore before any face recognition runs ;-)
);

