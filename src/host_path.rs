// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

use ashpd::documents::{DocumentID, Documents};
use fotema_core::FlatpakPathBuf;
use regex::Regex;
use std::path::Path;
use tracing::debug;

/// Derive host path from sandbox document path.
pub async fn host_path(sandbox_path: &Path) -> Option<FlatpakPathBuf> {
    // If sandbox_path doesn't start with "/run/user/" then
    // it is a host path.
    if !sandbox_path.starts_with("/run/user/") {
        return Some(FlatpakPathBuf::build(sandbox_path, sandbox_path));
    }

    // Parse Document ID from file chooser path.
    let doc_id: Option<DocumentID> = sandbox_path
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
        host_paths
            .get(&doc_id)
            .map(|file_path| FlatpakPathBuf::build(file_path.as_ref().to_path_buf(), sandbox_path))
    } else {
        None
    }
}
