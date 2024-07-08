use candle_core::{safetensors, DType, Device, Result, Tensor};
/*
use face_tracking_rs::blaze_face::{
    blaze_face::{BlazeFace, ModelType},
    face_detection::FaceDetection,
};
*/

use super::{blaze_face::BlazeFace, blaze_face::ModelType, face_detection::FaceDetection};
use image::{DynamicImage, Rgba, RgbaImage};

pub fn load_model(
    model_type: ModelType,
    min_score_threshold: f32,
    min_suppression_threshold: f32,
    device: &Device,
) -> Result<BlazeFace> {
    let dtype = DType::F16;
    let safetensors_path = match model_type {
        ModelType::Back => "src/blaze_face/data/blazefaceback.safetensors",
        ModelType::Front => "src/blaze_face/data/blazeface.safetensors",
    };
    let safetensors = safetensors::load(safetensors_path, device)?;

    // Load the variables
    let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, device);

    let anchor_path = match model_type {
        ModelType::Back => "src/blaze_face/data/anchorsback.npy",
        ModelType::Front => "src/blaze_face/data/anchors.npy",
    };

    // Load the anchors
    let anchors = Tensor::read_npy(anchor_path)? // (896, 4)
        .to_dtype(dtype)?
        .to_device(device)?;

    // Load the model
    BlazeFace::load(
        model_type,
        &variables,
        anchors,
        100.,
        min_score_threshold,
        min_suppression_threshold,
    )
}

pub fn load_image(image_path: &str, model_type: ModelType) -> anyhow::Result<DynamicImage> {
    let image = image::open(image_path)?;
    let image = match model_type {
        ModelType::Back => image.resize_to_fill(256, 256, image::imageops::FilterType::Nearest),
        ModelType::Front => image.resize_to_fill(128, 128, image::imageops::FilterType::Nearest),
    };

    Ok(image)
}

pub fn convert_image_to_tensor(image: &DynamicImage, device: &Device) -> Result<Tensor> {
    let pixels = image.to_rgb32f().to_vec();

    Tensor::from_vec(
        pixels,
        (image.width() as usize, image.height() as usize, 3),
        device,
    )? // (width, height, channel = 3) in range [0., 1.]
    .permute((2, 1, 0))? // (3, height, width) in range [0., 1.]
    .contiguous()?
    .broadcast_mul(&Tensor::from_slice(&[2_f32], 1, device)?)? // (3, height, width) in range [0., 2.]
    .broadcast_sub(&Tensor::from_slice(&[1_f32], 1, device)?) // (3, height, width) in range [-1., 1.]
}
