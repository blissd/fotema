// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use ashpd::documents::{DocumentID, Documents};
use regex::Regex;
use std::path::{Path, PathBuf};
use tracing::debug;

/// Derive host path from sandbox document path.
pub async fn host_path(pic_base_dir: &Path) -> Option<PathBuf> {
    // Parse Document ID from file chooser path.
    let doc_id: Option<DocumentID> = pic_base_dir
        .to_str()
        .and_then(|s| {
            let re = Regex::new(r"^/run/user/[0-9]+/doc/([0-9a-fA-F]+)/").unwrap();
            re.captures(s)
        })
        .and_then(|re_match| re_match.get(1))
        .map(|doc_id_match| doc_id_match.as_str().into());

    if let Some(doc_id) = doc_id {
        debug!("Document ID={:?}", doc_id);
        let proxy = Documents::new().await.unwrap();
        let host_paths = proxy.host_paths(&[doc_id.clone()]).await.unwrap();
        let host_path = host_paths
            .get(&doc_id)
            .map(|file_path| file_path.as_ref().to_path_buf());
        host_path
    } else {
        None
    }
}
