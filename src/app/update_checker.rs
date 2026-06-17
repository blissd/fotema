// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Checks GitHub for a newer Fotema release.
//!
//! Uses a blocking `ureq` request so it must be run off the UI thread
//! (e.g. via `relm4::spawn_blocking`), avoiding any async-runtime/reactor
//! coupling with the glib main loop.

use std::time::Duration;

use serde::Deserialize;
use tracing::warn;

const GITHUB_API: &str = "https://api.github.com/repos/blissd/fotema/releases/latest";

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
}

/// Returns `Some(latest_version)` if GitHub reports a release newer than
/// `installed`, otherwise `None` (also on any network/parse error).
pub fn check(installed: &str) -> Option<String> {
    let user_agent = format!("Fotema/{installed}");

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .user_agent(user_agent)
        .build()
        .into();

    let release: GitHubRelease = match agent.get(GITHUB_API).call() {
        Ok(mut resp) => match resp.body_mut().read_json() {
            Ok(release) => release,
            Err(e) => {
                warn!("Failed to parse GitHub release response: {e}");
                return None;
            }
        },
        Err(e) => {
            warn!("Update check request failed: {e}");
            return None;
        }
    };

    let latest = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name);

    if is_newer(latest, installed) {
        Some(latest.to_string())
    } else {
        None
    }
}

/// Numeric, segment-wise semver comparison. Missing segments count as 0, so
/// patch releases (2.4.2 -> 2.4.3) are detected, not just major/minor.
fn is_newer(latest: &str, installed: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|p| p.parse().ok()).collect() };
    let latest = parse(latest);
    let installed = parse(installed);

    let len = latest.len().max(installed.len());
    for i in 0..len {
        let l = latest.get(i).copied().unwrap_or(0);
        let n = installed.get(i).copied().unwrap_or(0);
        if l != n {
            return l > n;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn detects_newer_versions() {
        assert!(is_newer("2.5.0", "2.4.2"));
        assert!(is_newer("2.4.3", "2.4.2")); // patch update detected
        assert!(is_newer("3.0.0", "2.9.9"));
        assert!(is_newer("2.4", "2.3.9"));
    }

    #[test]
    fn rejects_same_or_older() {
        assert!(!is_newer("2.4.2", "2.4.2"));
        assert!(!is_newer("2.4.1", "2.4.2"));
        assert!(!is_newer("2.4.2", "2.4.2.0"));
        assert!(!is_newer("1.0.0", "2.0.0"));
    }
}
