-- SPDX-FileCopyrightText: © 2024 David Bliss
--
-- SPDX-License-Identifier: GPL-3.0-or-later

-- ArcFace recognition embedding (a vector of f32, stored little-endian) for
-- each detected face. Used by the fast recognition pass, which compares the
-- stored vectors instead of running the model for every face.
ALTER TABLE pictures_faces ADD COLUMN embedding BLOB;
