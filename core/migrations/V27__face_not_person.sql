-- Negative learning: records that a face was explicitly rejected as a given
-- person ("this is NOT person X"). The recognition pass must never re-assign a
-- face to a person it was rejected for, which stops similar-looking people
-- (e.g. relatives) from being repeatedly confused.
CREATE TABLE face_not_person (
    face_id   INTEGER NOT NULL,
    person_id INTEGER NOT NULL,
    PRIMARY KEY (face_id, person_id)
);

CREATE INDEX ix_face_not_person_face_id ON face_not_person (face_id);
