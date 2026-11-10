// SPDX-FileCopyrightText: © 2025 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Shared downloader for machine learning models. Face recognition fetches the
// SFace alignment model and the ArcFace embedding model on first use. This
// module holds the one implementation that both downloads use.
//
// The download must be correct. A truncated model file looks valid on disk and
// stays in the cache forever. Therefore this module does three things:
//
// 1. It streams the body into a temporary file and checks the write result.
// 2. It compares the SHA-256 of the written bytes with the expected value.
// 3. It renames the temporary file to the final name only after the check.
//
// The temporary file carries the process id and a counter. Thus two processes
// never write into the same temporary file.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use sha2::{Digest, Sha256};
use tracing::{info, warn};

/// Time limit for the connection setup. The transfer itself has no limit,
/// because a model file is large and a slow line is still a valid line.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// Time limit for the whole transfer. The blocking client of reqwest has no
/// limit for a single read. Therefore the limit covers the complete download.
/// The value must be large: the text encoder has 1.7 GB.
const TRANSFER_TIMEOUT: Duration = Duration::from_secs(3 * 60 * 60);

/// Upper limit for HTTP redirects. HuggingFace redirects to a CDN host.
const MAX_REDIRECTS: usize = 10;

/// Counter for unique temporary file names inside one process.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A model file that the application downloads on first use.
pub struct ModelFile {
    /// Source address of the file.
    pub url: &'static str,
    /// Expected SHA-256 of the content, as lowercase hexadecimal.
    pub sha256: &'static str,
    /// Expected size in bytes.
    pub size: u64,
    /// Text for the log messages.
    pub description: &'static str,
}

/// Ensure the model file is present at `destination` and correct.
///
/// The function returns at once if the file exists and matches the hash. If the
/// file exists but does not match, the function deletes it and downloads again.
pub fn ensure_model(model: &ModelFile, destination: &Path) -> Result<()> {
    if destination.exists() {
        match verify_file(destination, model) {
            Ok(()) => return Ok(()),
            Err(e) => {
                warn!(
                    "Cached model ({}) at {} is invalid ({}); downloading again.",
                    model.description,
                    destination.display(),
                    e
                );
                std::fs::remove_file(destination)?;
            }
        }
    }

    download(model, destination)
}

/// Compare the file on disk with the expected size and hash.
fn verify_file(path: &Path, model: &ModelFile) -> Result<()> {
    let actual_size = std::fs::metadata(path)?.len();
    if actual_size != model.size {
        return Err(anyhow!(
            "size is {} bytes, expected {} bytes",
            actual_size,
            model.size
        ));
    }

    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    let actual = hex_string(&hasher.finalize());

    if actual != model.sha256 {
        return Err(anyhow!("SHA-256 is {}, expected {}", actual, model.sha256));
    }

    Ok(())
}

/// Download the file, check it, and then move it to its final name.
fn download(model: &ModelFile, destination: &Path) -> Result<()> {
    info!(
        "Downloading model ({}) from {}",
        model.description, model.url
    );

    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("*/*"));

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(TRANSFER_TIMEOUT)
        .redirect(Policy::limited(MAX_REDIRECTS))
        .build()?;

    let mut response = client.get(model.url).headers(headers).send()?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download model ({}): {}",
            model.description,
            response.status()
        ));
    }

    let tmp_path = temp_path(destination);

    // The closure writes the body and returns size and hash. A failure must not
    // leave the temporary file behind, so the caller removes it on every error.
    let result = write_body(&mut response, &tmp_path);

    let (written, digest) = match result {
        Ok(v) => v,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
    };

    if written != model.size {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(anyhow!(
            "Model ({}) is incomplete: got {} bytes, expected {} bytes",
            model.description,
            written,
            model.size
        ));
    }

    if digest != model.sha256 {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(anyhow!(
            "Model ({}) is corrupt: SHA-256 is {}, expected {}",
            model.description,
            digest,
            model.sha256
        ));
    }

    std::fs::rename(&tmp_path, destination)?;
    info!("Model ({}) downloaded and verified.", model.description);
    Ok(())
}

/// Stream the response body into `tmp_path`. Return the byte count and the
/// SHA-256 as lowercase hexadecimal.
fn write_body(
    response: &mut reqwest::blocking::Response,
    tmp_path: &Path,
) -> Result<(u64, String)> {
    let tmp_file = File::create(tmp_path)?;
    let mut writer = BufWriter::new(tmp_file);
    let mut hasher = Sha256::new();

    // Read the body once, in blocks. `copy_to` must not run in a loop: the first
    // call consumes the whole body, and a later call only returns 0.
    let mut buffer = vec![0u8; 256 * 1024];
    let mut written: u64 = 0;
    loop {
        let n = std::io::Read::read(response, &mut buffer)?;
        if n == 0 {
            break;
        }
        writer.write_all(&buffer[..n])?;
        hasher.update(&buffer[..n]);
        written += n as u64;
    }

    // Flush the buffer and the file. Without these steps the rename can publish
    // a truncated file.
    writer.flush()?;
    let file = writer
        .into_inner()
        .map_err(|e| anyhow!("Failed to flush model file: {e}"))?;
    file.sync_all()?;

    Ok((written, hex_string(&hasher.finalize())))
}

/// Build a temporary file name that is unique per process and per call.
fn temp_path(destination: &Path) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let name = destination
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "model".to_string());
    let tmp_name = format!("{}.{}.{}.tmp", name, std::process::id(), counter);
    destination.with_file_name(tmp_name)
}

/// Format bytes as lowercase hexadecimal.
fn hex_string(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{:02x}", b);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_string_formats_lowercase_with_padding() {
        assert_eq!(hex_string(&[0x00, 0x0f, 0xff, 0xa5]), "000fffa5");
    }

    #[test]
    fn temp_path_is_unique_per_call() {
        let destination = Path::new("/tmp/models/model.onnx");
        let a = temp_path(destination);
        let b = temp_path(destination);
        assert_ne!(a, b);
        assert_eq!(a.parent(), destination.parent());
    }

    #[test]
    fn verify_file_detects_wrong_size() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"abc").unwrap();

        let model = ModelFile {
            url: "https://example.invalid/model.bin",
            // SHA-256 of "abc".
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            size: 4,
            description: "test model",
        };

        assert!(verify_file(&path, &model).is_err());
    }

    #[test]
    fn verify_file_accepts_matching_content() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"abc").unwrap();

        let model = ModelFile {
            url: "https://example.invalid/model.bin",
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            size: 3,
            description: "test model",
        };

        verify_file(&path, &model).unwrap();
    }

    #[test]
    fn verify_file_detects_wrong_hash() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.bin");
        std::fs::write(&path, b"abd").unwrap();

        let model = ModelFile {
            url: "https://example.invalid/model.bin",
            sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            size: 3,
            description: "test model",
        };

        assert!(verify_file(&path, &model).is_err());
    }
}
