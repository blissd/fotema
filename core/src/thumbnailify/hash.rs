// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use md5::{Digest, Md5};
use tracing::debug;

/// Computes the MD5 hash for the given input file path.
/// `input` will be a `file:///...` URI to the host path of the file.
pub fn compute_hash(input: &str) -> String {
    debug!("Computing MD5 hash for input: {}", input);
    let mut hasher = Md5::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    let hash = format!("{:x}", result);

    debug!("MD5 hash for input={} is {}", input, hash);
    hash
}
