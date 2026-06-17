// SPDX-FileCopyrightText: © 2024 David Bliss
//
// SPDX-License-Identifier: GPL-3.0-or-later
//
// Face embeddings use an ArcFace model (InsightFace `buffalo_l` / w600k_r50,
// 512-d) run through OpenCV's DNN module. ArcFace is far more discriminative
// than the previous SFace model, which matters for telling similar-looking
// people (e.g. relatives) apart. SFace's `align_crop` is still used to produce
// the 112x112 aligned face from the 5 landmarks (the alignment template matches
// what ArcFace expects); only the embedding network changed.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

use opencv::core::Mat;
use opencv::imgcodecs;
use opencv::objdetect::FaceRecognizerSF;
use opencv::prelude::*;

use reqwest::header::{ACCEPT, HeaderMap, HeaderValue};

use tracing::{info, warn};

use crate::people::model::{DetectedFace, PersonForRecognition, PersonId};

/// ArcFace embedding dimension (w600k_r50 outputs 512 floats). The byte length
/// of a stored embedding (512 * 4 = 2048) distinguishes an ArcFace embedding
/// from an older 128-d SFace one, so old embeddings are recomputed.
pub const EMBEDDING_DIM: usize = 512;

const SFACE_URL: &str = "https://github.com/blissd/fotema-opencv_zoo/raw/fotema-1.0/models/face_recognition_sface/face_recognition_sface_2021dec.onnx";
const SFACE_FILE: &str = "face_recognition_sface_2021dec.onnx";

const ARCFACE_URL: &str =
    "https://huggingface.co/immich-app/buffalo_l/resolve/main/recognition/model.onnx";
const ARCFACE_FILE: &str = "face_recognition_arcface_r50.onnx";

/// Per-thread face embedder: an SFace recognizer used only for landmark-based
/// alignment, plus the ArcFace DNN that produces the embedding. Not thread-safe
/// (neither OpenCV component is), so create one per worker thread.
pub struct FaceEmbedder {
    aligner: opencv::core::Ptr<FaceRecognizerSF>,
    net: opencv::dnn::Net,
}

impl FaceEmbedder {
    pub fn new(sface_path: &Path, arcface_path: &Path) -> Result<Self> {
        let aligner = new_aligner(sface_path)?;
        let net = new_arcface_net(arcface_path)?;
        Ok(Self { aligner, net })
    }

    /// Compute the L2-normalised ArcFace embedding for a detected face.
    pub fn embedding(&mut self, face: &DetectedFace) -> Result<Vec<f32>> {
        let face_img = imgcodecs::imread_def(&face.face_path.to_string_lossy())?;
        if face_img.empty() {
            return Err(anyhow!(
                "Face crop unreadable (empty image): {:?}",
                face.face_path
            ));
        }

        // Align to the canonical 112x112 using the 5 landmarks.
        let landmarks = face.landmarks_as_mat();
        let mut aligned = Mat::default();
        self.aligner.align_crop(&face_img, &landmarks, &mut aligned)?;

        // ArcFace preprocessing: (pixel - 127.5) / 127.5, RGB, 112x112.
        let blob = opencv::dnn::blob_from_image(
            &aligned,
            1.0 / 127.5,
            opencv::core::Size::new(112, 112),
            opencv::core::Scalar::new(127.5, 127.5, 127.5, 0.0),
            true,  // swap BGR -> RGB
            false, // no crop
            opencv::core::CV_32F,
        )?;

        self.net
            .set_input(&blob, "", 1.0, opencv::core::Scalar::default())?;
        let out = self.net.forward_single_def()?;
        let data: &[f32] = out.data_typed()?;

        let mut v = data.to_vec();
        normalize(&mut v);
        Ok(v)
    }
}

/// Matches unknown faces against named people using their ArcFace embeddings.
pub struct FaceRecognizer {
    /// (person, normalised ArcFace embedding) for each named reference face.
    people: Vec<(PersonForRecognition, Vec<f32>)>,
}

impl FaceRecognizer {
    /// Cosine similarity above which an unknown face is taken to be a named
    /// person. ArcFace separates identities well; this leans towards precision
    /// (fewer false merges of similar-looking people).
    const COSINE_THRESHOLD: f32 = 0.42;

    /// Build a recognizer by computing each named person's reference embedding.
    pub fn build(cache_dir: &Path, people: Vec<PersonForRecognition>) -> Result<Self> {
        let (sface_path, arcface_path) = Self::ensure_models(cache_dir)?;
        let mut embedder = FaceEmbedder::new(&sface_path, &arcface_path)?;

        let mut refs = Vec::with_capacity(people.len());
        for person in people {
            match embedder.embedding(&person.face) {
                Ok(e) => refs.push((person, e)),
                Err(e) => warn!(
                    "Skipping person {} reference (unreadable): {:?}",
                    person.person_id, e
                ),
            }
        }
        Ok(Self { people: refs })
    }

    /// Best matching named person for an unknown face, or None if none is close
    /// enough. `embedder` is reused across faces on the calling thread.
    pub fn recognize(
        &self,
        embedder: &mut FaceEmbedder,
        unknown_face: &DetectedFace,
    ) -> Result<Option<PersonId>> {
        let unknown = embedder.embedding(unknown_face)?;

        let best = self
            .people
            .iter()
            // Only recognise against people known before this face was detected.
            .filter(|(p, _)| p.recognized_at <= unknown_face.detected_at)
            .map(|(person, reference)| (person, cosine(&unknown, reference)))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((person, score)) = best {
            if score >= Self::COSINE_THRESHOLD {
                return Ok(Some(person.person_id));
            }
        }
        Ok(None)
    }

    /// Ensure both the SFace (alignment) and ArcFace (embedding) models are
    /// present, returning their paths.
    pub fn ensure_models(cache_dir: &Path) -> Result<(PathBuf, PathBuf)> {
        let base_path = cache_dir.join("opencv_models");
        std::fs::create_dir_all(&base_path)?;

        let sface = base_path.join(SFACE_FILE);
        download_model(SFACE_URL, &sface, "SFace alignment model (~40MB)")?;

        let arcface = base_path.join(ARCFACE_FILE);
        download_model(ARCFACE_URL, &arcface, "ArcFace r50 model (~166MB)")?;

        Ok((sface, arcface))
    }
}

/// Load the SFace recognizer (used only for `align_crop`), preferring OpenCL.
fn new_aligner(model_path: &Path) -> Result<opencv::core::Ptr<FaceRecognizerSF>> {
    let model = model_path.to_string_lossy();
    if opencv::core::have_opencl().unwrap_or(false) {
        if let Ok(r) = FaceRecognizerSF::create(
            &model,
            "",
            opencv::dnn::DNN_BACKEND_OPENCV,
            opencv::dnn::DNN_TARGET_OPENCL,
        ) {
            return Ok(r);
        }
    }
    Ok(FaceRecognizerSF::create(
        &model,
        "",
        opencv::dnn::DNN_BACKEND_OPENCV,
        opencv::dnn::DNN_TARGET_CPU,
    )?)
}

/// Load the ArcFace ONNX into an OpenCV DNN net, preferring OpenCL.
fn new_arcface_net(model_path: &Path) -> Result<opencv::dnn::Net> {
    let mut net = opencv::dnn::read_net_from_onnx(&model_path.to_string_lossy())?;
    net.set_preferable_backend(opencv::dnn::DNN_BACKEND_OPENCV)?;
    let target = if opencv::core::have_opencl().unwrap_or(false) {
        info!("ArcFace recognizer using OpenCL (GPU) acceleration.");
        opencv::dnn::DNN_TARGET_OPENCL
    } else {
        info!("ArcFace recognizer using CPU.");
        opencv::dnn::DNN_TARGET_CPU
    };
    net.set_preferable_target(target)?;
    Ok(net)
}

/// L2-normalise a vector in place.
fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Cosine similarity of two equal-length, L2-normalised vectors (= dot product).
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return -1.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn download_model(url: &str, destination: &Path, description: &str) -> Result<()> {
    if destination.exists() {
        return Ok(());
    }

    info!("Downloading face recognition model ({}) from {}", description, url);

    let headers = {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers
    };

    let client = reqwest::blocking::Client::new();
    let mut response = client.get(url).headers(headers).send()?;

    if response.status().is_success() {
        let tmp_path = destination.with_extension("tmp");
        let tmp_file = File::create(&tmp_path)?;
        let mut writer = BufWriter::new(tmp_file);
        while let Ok(bytes_read) = response.copy_to(&mut writer) {
            if bytes_read == 0 {
                break;
            }
        }
        info!("Face recognition model ({}) downloaded.", description);
        std::fs::rename(tmp_path, destination)?;
        Ok(())
    } else {
        Err(anyhow!(
            "Failed to download face recognition model ({}): {}",
            description,
            response.status()
        ))
    }
}
