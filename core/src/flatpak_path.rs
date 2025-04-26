// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::thumbnailify;
use std::path::PathBuf;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FlatpakPathBuf {
    pub host_path: PathBuf,
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

    pub fn exists(&self) -> bool {
        self.sandbox_path.exists()
    }
}
