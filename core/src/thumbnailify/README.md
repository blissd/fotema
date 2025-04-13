<!--
SPDX-FileCopyrightText: © 2024-2025 David Bliss

SPDX-License-Identifier: GFDL-1.3-or-later
-->
# Thumbnailify

This files in this directory are from Thumbnailify, which is on
[GitHub](https://github.com/luigi311/thumbnailify/tree/main).

Thumbnailify uses a GPl3 licence.

## Why import Thumbnailify?

Fotema runs in a Flatpak sandbox and does not have read access to
files on the filesystem outside of the sandbox. For example, a user's
photo library at `~/Pictures` is mounted in Fotema to
`/run/$UID/docs/$DOC_ID/Pictures`.

When generating a thumbnail the host path (the path outside of the sandbox)
must be used when generating the URI when deriving the MD5 hash, but the
sandbox path must be used reading the file size, modification date, and so on.

This import of Thumbnailify makes some small tweaks to allow this usage.

