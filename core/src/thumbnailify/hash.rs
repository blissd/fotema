// SPDX-FileCopyrightText: © 2025 luigi311 <git@luigi311.com>
// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use md5;
use std::path::Path;
use tracing::debug;

/// Computes the MD5 hash for the given input file path.
/// `input` will be a `file:///...` URI to the host path of the file.
pub fn compute_hash(input: &str) -> String {
    debug!("Computing MD5 hash for input: {}", input);
    let digest = md5::compute(input.as_bytes());
    let hash = format!("{:x}", digest);

    debug!("MD5 hash for input={} is {}", input, hash);
    hash
}

pub fn compute_hash_for_path(host_path: &Path) -> String {
    let file_uri = super::file::get_file_uri(host_path).unwrap();
    compute_hash(&file_uri)
}
