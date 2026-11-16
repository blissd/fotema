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

use std::path::Path;
use std::path::PathBuf;

use anyhow::{Result, anyhow};

use opencv::core::Mat;
use opencv::imgcodecs;
use opencv::objdetect::FaceRecognizerSF;
use opencv::prelude::*;

use crate::machine_learning::model_download::{ModelFile, ensure_model};

use tracing::{info, warn};

use crate::people::model::DetectedFace;

/// ArcFace embedding dimension (w600k_r50 outputs 512 floats). The byte length
/// of a stored embedding (512 * 4 = 2048) distinguishes an ArcFace embedding
/// from an older 128-d SFace one, so old embeddings are recomputed.
pub const EMBEDDING_DIM: usize = 512;

// Each model file carries its expected size and SHA-256. The SFace values come
// from the file in the GitHub repository, the ArcFace values from the
// HuggingFace headers `x-linked-size` and `x-linked-etag` (revision `main`,
// 2026-09-20). If you change a URL, update size and hash together.
const SFACE_MODEL: ModelFile = ModelFile {
    url: "https://github.com/blissd/fotema-opencv_zoo/raw/fotema-1.0/models/face_recognition_sface/face_recognition_sface_2021dec.onnx",
    sha256: "0ba9fbfa01b5270c96627c4ef784da859931e02f04419c829e83484087c34e79",
    size: 38_696_353,
    description: "SFace alignment model (~40MB)",
};
const SFACE_FILE: &str = "face_recognition_sface_2021dec.onnx";

const ARCFACE_MODEL: ModelFile = ModelFile {
    url: "https://huggingface.co/immich-app/buffalo_l/resolve/main/recognition/model.onnx",
    sha256: "4c06341c33c2ca1f86781dab0e829f88ad5b64be9fba56e56bc9ebdefc619e43",
    size: 174_383_860,
    description: "ArcFace r50 model (~166MB)",
};
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
        let face_img = imgcodecs::imread_def(&face.face_path)?;
        if face_img.empty() {
            return Err(anyhow!(
                "Face crop unreadable (empty image): {:?}",
                face.face_path
            ));
        }

        // Align to the canonical 112x112 using the 5 landmarks.
        let landmarks = face.landmarks_as_mat();
        let mut aligned = Mat::default();
        self.aligner
            .align_crop(&face_img, &landmarks, &mut aligned)?;

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

impl FaceEmbedder {
    /// Ensure both the SFace (alignment) and ArcFace (embedding) models are
    /// present, returning their paths.
    pub fn ensure_models(cache_dir: &Path) -> Result<(PathBuf, PathBuf)> {
        let base_path = cache_dir.join("opencv_models");
        std::fs::create_dir_all(&base_path)?;

        let sface = base_path.join(SFACE_FILE);
        ensure_model(&SFACE_MODEL, &sface)?;

        let arcface = base_path.join(ARCFACE_FILE);
        ensure_model(&ARCFACE_MODEL, &arcface)?;

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

/// Load the ArcFace ONNX into an OpenCV DNN net, preferring OpenCL but only if a
/// warmup inference actually succeeds. A vendor-neutral OpenCL stack like Mesa
/// Rusticl can report a device yet fail on some DNN op only at inference time, so
/// validating up front lets us fall back to CPU cleanly instead of failing every
/// face embedding.
fn new_arcface_net(model_path: &Path) -> Result<opencv::dnn::Net> {
    let read = |target: i32| -> Result<opencv::dnn::Net> {
        // In opencv-rust 0.100 the function read_net_from_onnx has a new
        // parameter `engine`. The _def variant keeps ENGINE_AUTO. The function
        // accepts impl AsRef<OsStr>. Thus you can give it the path directly.
        let mut net = opencv::dnn::read_net_from_onnx_def(model_path)?;
        net.set_preferable_backend(opencv::dnn::DNN_BACKEND_OPENCV)?;
        net.set_preferable_target(target)?;
        Ok(net)
    };

    if opencv::core::have_opencl().unwrap_or(false) {
        match read(opencv::dnn::DNN_TARGET_OPENCL) {
            Ok(mut net) => match arcface_warmup(&mut net) {
                Ok(()) => {
                    info!("ArcFace recognizer using OpenCL (GPU) acceleration.");
                    return Ok(net);
                }
                Err(e) => warn!("ArcFace OpenCL warmup failed ({e}); using CPU."),
            },
            Err(e) => warn!("ArcFace OpenCL init failed ({e}); using CPU."),
        }
    }

    let net = read(opencv::dnn::DNN_TARGET_CPU)?;
    info!("ArcFace recognizer using CPU.");
    Ok(net)
}

/// Run a single forward on a zeroed 112x112 input to force OpenCL kernel
/// compilation and surface any unsupported-op errors before real use.
fn arcface_warmup(net: &mut opencv::dnn::Net) -> Result<()> {
    let dummy = Mat::zeros(112, 112, opencv::core::CV_8UC3)?.to_mat()?;
    let blob = opencv::dnn::blob_from_image(
        &dummy,
        1.0 / 127.5,
        opencv::core::Size::new(112, 112),
        opencv::core::Scalar::new(127.5, 127.5, 127.5, 0.0),
        true,
        false,
        opencv::core::CV_32F,
    )?;
    net.set_input(&blob, "", 1.0, opencv::core::Scalar::default())?;
    let _ = net.forward_single_def()?;
    Ok(())
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
