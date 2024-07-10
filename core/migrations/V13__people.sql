

-- People named by the user who will faces associated with them
CREATE TABLE pictures_people (
        person_id      INTEGER PRIMARY KEY UNIQUE NOT NULL, -- unique ID for person

        thumbnail_path TEXT UNIQUE NOT NULL, -- path to primary person thumbnail
        name           TEXT UNIQUE NOT NULL -- name of person
);

