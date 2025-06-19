// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::thumbnailify;
use crate::thumbnailify::ThumbnailSize;
use std::path::{Path, PathBuf};

/// A path to a file that exists both inside and outside of the Flatpak sandbox.
/// FIXME does Default make sense? It is here to make Settings compile.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct FlatpakPathBuf {
    /// Path on the host system. This is the path to display in the UI and to use
    /// when computing thumbnail hashes.
    /// Fotema likely won't be able to read from this path.
    pub host_path: PathBuf,

    /// Path inside the sandbox, likely under `/run/user/$UID/docs/$DOC_ID/...`.
    /// This is the path to use when reading a file.
    pub sandbox_path: PathBuf,
}

impl FlatpakPathBuf {
    pub fn build(
        host_path: impl Into<PathBuf>,
        sandbox_path: impl Into<PathBuf>,
    ) -> FlatpakPathBuf {
        FlatpakPathBuf {
            host_path: host_path.into(),
            sandbox_path: sandbox_path.into(),
        }
    }

    pub fn thumbnail_hash(&self) -> String {
        thumbnailify::compute_hash_for_path(&self.host_path)
    }

    pub fn thumbnail_path(&self, thumbnails_base_dir: &Path, size: ThumbnailSize) -> PathBuf {
        thumbnailify::get_thumbnail_path(thumbnails_base_dir, &self.host_path, size)
    }

    pub fn exists(&self) -> bool {
        self.sandbox_path.exists()
    }
}
