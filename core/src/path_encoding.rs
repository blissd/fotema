// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

/// Operations for encoding paths to and from base64 strings.
/// Paths are not strings, so should not be saved to a TEXT column in the sqlite database.
/// The TEXT data type can be UTF8 or UTF16, which paths are _not_.
/// However, I tried to save paths as the BLOB type, but it was just too painiful so now
/// paths are encoded as base64. This allows non-UTF8 and non-UTF16 paths to be saved to a
/// TEXT column in sqlite.
///
/// Note that each base 64 column will hav a b64 suffix and will
/// also have a '*_lossy' sibling with a non-base 64 encoded version of the path with
/// invalid non-UTF8 characters removed. The lossy sibling is for debugging only and
/// should not be read by Fotema.
///
/// Also note that Fotema computes some relative paths, such as for thumbnails, and these
/// _won't_ be base 64 encoded as we can be sure to only use UTF8 characters in the paths.
use anyhow::*;
use base64::prelude::*;
use std::ffi::OsString;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

/// Encode a path as a base 64 string.
pub fn to_base64(p: &Path) -> String {
    BASE64_STANDARD.encode(p.as_os_str().as_bytes())
}

pub fn from_base64(s: &String) -> Result<PathBuf> {
    Ok(BASE64_STANDARD
        .decode(s)
        .map(OsString::from_vec)
        .map(PathBuf::from)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode() {
        let path = Path::new("this/is/a/path.jpg");
        let path_b64 = to_base64(&path);

        assert_eq!("dGhpcy9pcy9hL3BhdGguanBn".to_string(), path_b64);

        let decoded_path = from_base64(&path_b64).unwrap();
        assert_eq!(decoded_path, path);
    }
}
