-- SPDX-FileCopyrightText: © 2024 David Bliss
--
-- SPDX-License-Identifier: GPL-3.0-or-later

-- SFace recognition embedding (a vector of f32, stored little-endian) for each
-- detected face. Used to cluster unnamed faces so the "unknown people" view can
-- group and order them by how often the same person appears.
ALTER TABLE pictures_faces ADD COLUMN embedding BLOB;
