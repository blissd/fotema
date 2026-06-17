-- Allow a whole person to be ignored (hidden from the people overview and from
-- auto-recognition) without losing data. Reversible: clearing the flag restores
-- the person and all their face assignments. Additive + defaulted, so older
-- databases adopt it transparently.
ALTER TABLE people ADD COLUMN is_ignored BOOLEAN NOT NULL DEFAULT 0;
