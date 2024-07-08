// SPDX-FileCopyrightText: © 2023 Mochineko <t.o.e.4315@gmail.com>
//
// SPDX-License-Identifier: MIT
//
// Reference implementation:
// https://github.com/hollance/BlazeFace-PyTorch/blob/master/blazeface.py

use candle_core::{DType, Error, IndexOp, Result, Shape, Tensor};
use candle_nn::{ops, VarBuilder};

use crate::face::blaze_face::{
    blaze_face_back_model::BlazeFaceBackModel,
    blaze_face_config::{BlazeFaceConfig, DTYPE_IN_BLAZE_FACE},
    blaze_face_front_model::BlazeFaceFrontModel,
    non_max_suppression,
};

#[derive(Clone, Copy, Debug)]
pub enum ModelType {
    Back,
    Front,
}

pub(crate) trait BlazeFaceModel {
    fn forward(&self, xs: &Tensor) -> Result<(Tensor, Tensor)>;
}

pub struct BlazeFace {
    model: Box<dyn BlazeFaceModel>,
    anchors: Tensor,
    config: BlazeFaceConfig,
}

impl BlazeFace {
    pub fn load(
        model_type: ModelType,
        variables: &VarBuilder,
        anchors: Tensor,
        score_clipping_thresh: f32,
        min_score_thresh: f32,
        min_suppression_threshold: f32,
    ) -> Result<Self> {
        let device = variables.device();
        if !device.same_device(anchors.device()) {
            return Result::Err(Error::DeviceMismatchBinaryOp {
                lhs: device.location(),
                rhs: anchors.device().location(),
                op: "load_blaze_face",
            });
        }
        if anchors.dims() != [896, 4] {
            return Result::Err(Error::ShapeMismatchBinaryOp {
                lhs: anchors.shape().clone(),
                rhs: Shape::from_dims(&[896, 4]),
                op: "load_blaze_face",
            });
        }

        if variables.dtype() != DTYPE_IN_BLAZE_FACE {
            return Result::Err(Error::DTypeMismatchBinaryOp {
                lhs: variables.dtype(),
                rhs: DTYPE_IN_BLAZE_FACE,
                op: "load_blaze_face",
            });
        }

        // NOTE: Enforce the dtype of the anchors and variables to be DType::F16.
        let anchors = anchors.to_dtype(DType::F16)?;

        match model_type {
            ModelType::Back => {
                let model = BlazeFaceBackModel::load(variables)?;
                Ok(BlazeFace {
                    model: Box::new(model),
                    anchors,
                    config: BlazeFaceConfig::back(
                        score_clipping_thresh,
                        min_score_thresh,
                        min_suppression_threshold,
                        device,
                    )?,
                })
            }
            ModelType::Front => {
                let model = BlazeFaceFrontModel::load(variables)?;
                Ok(BlazeFace {
                    model: Box::new(model),
                    anchors,
                    config: BlazeFaceConfig::front(
                        score_clipping_thresh,
                        min_score_thresh,
                        min_suppression_threshold,
                        device,
                    )?,
                })
            }
        }
    }

    pub fn forward(
        &self,
        images: &Tensor, // back:(batch_size, 3, 256, 256) or front:(batch_size, 3, 128, 128)
    ) -> Result<(Tensor, Tensor)> // coordinates:(batch, 896, 16), score:(batch, 896, 1)
    {
        self.model.forward(images)
    }

    pub fn predict_on_image(
        &self,
        image: &Tensor, // (3, 256, 256) or (3, 128, 128)
    ) -> Result<Vec<Vec<Tensor>>> // Vec<(detected_faces, 17)> with length:batch_size
    {
        self.predict_on_batch(&image.unsqueeze(0)?)
    }

    pub fn predict_on_batch(
        &self,
        images: &Tensor, // (batch_size, 3, 256, 256) or (batch_size, 3, 128, 128)
    ) -> Result<Vec<Vec<Tensor>>> // Vec<(detected_faces, 17)> with length:batch_size
    {
        let (raw_boxes, raw_scores) = self.forward(images)?; // coordinates:(batch, 896, 16), score:(batch, 896, 1)

        let detections =
            tensors_to_detections(&raw_boxes, &raw_scores, &self.anchors, &self.config)?; // Vec<(num_detections, 17)> with length:batch_size

        let mut filtered_detections = Vec::new();
        for detection in detections {
            let faces = non_max_suppression::weighted_non_max_suppression(
                &detection.contiguous()?,
                self.config.min_suppression_threshold,
            )?; // Vec<(17)> with length:detected_faces
            if !faces.is_empty() {
                filtered_detections.push(faces);
            } else {
                filtered_detections.push(Vec::new());
            }
        }

        Ok(filtered_detections) // Vec<(detected_faces, 17)> with length:batch_size
    }
}

fn tensors_to_detections(
    raw_boxes: &Tensor,  // (batch_size, 896, 16)
    raw_scores: &Tensor, // (batch_size, 896, 1)
    anchors: &Tensor,    // (896, 4)
    config: &BlazeFaceConfig,
) -> Result<Vec<Tensor>> // Vec<(num_detections, 17)> with length:batch_size
{
    let detection_boxes = decode_boxes(raw_boxes, anchors, config)?; // (batch_size, 896, 16)

    raw_scores.clamp(
        -config.score_clipping_threshold,
        config.score_clipping_threshold,
    )?;

    let detection_scores = ops::sigmoid(raw_scores)?; // (batch_size, 896, 1)

    let indices = unmasked_indices(&detection_scores, config.min_score_threshold)?; // (batch_size, num_detections)

    let mut output = Vec::new();
    for batch in 0..raw_boxes.dims()[0] {
        // Filtering
        let boxes = detection_boxes.i((batch, &indices.i((batch, ..))?, ..))?; // (num_detections, 16)
        let scores = detection_scores.i((batch, &indices.i((batch, ..))?, ..))?; // (num_detections, 1)

        if boxes.dims()[0] == 0 || scores.dims()[0] == 0 {
            output.push(Tensor::zeros(
                (0, 17),
                raw_boxes.dtype(),
                raw_boxes.device(),
            )?);
        } else {
            let detection = Tensor::cat(&[boxes, scores], 1)?; // (896, 17)
            output.push(detection);
        }
    }

    Ok(output) // Vec<(num_detections, 17)> with length:batch_size
}

fn decode_boxes(
    raw_boxes: &Tensor, // (batch_size, 896, 16)
    anchors: &Tensor,   // (896, 4)
    config: &BlazeFaceConfig,
) -> Result<Tensor> // (batch_size, 896, 16)
{
    let mut coordinates = Vec::new();
    let two = Tensor::from_slice(&[2.], 1, raw_boxes.device())? // (1)
        .to_dtype(DTYPE_IN_BLAZE_FACE)?;

    // NOTE: Fix the order of the coordinates from original implementation,
    // because the tensor shape is (batch, channels, height, witdh) then y -> x in PyTorch.
    let y_anchor = anchors.i((.., 0))?; // (896)
    let x_anchor = anchors.i((.., 1))?; // (896)
    let h_anchor = anchors.i((.., 2))?; // (896)
    let w_anchor = anchors.i((.., 3))?; // (896)

    let y_center = raw_boxes
        .i((.., .., 0))?
        .broadcast_div(&config.y_scale)?
        .broadcast_mul(&h_anchor)?
        .broadcast_add(&y_anchor)?;

    let x_center = raw_boxes
        .i((.., .., 1))? // (batch_size, 896)
        .broadcast_div(&config.x_scale)? // / (1)
        .broadcast_mul(&w_anchor)? // * (896)
        .broadcast_add(&x_anchor)?; // + (896)
                                    // = (batch_size, 896)

    let h = raw_boxes
        .i((.., .., 2))?
        .broadcast_div(&config.h_scale)?
        .broadcast_mul(&h_anchor)?;

    let w = raw_boxes
        .i((.., .., 3))? // (batch_size, 896)
        .broadcast_div(&config.w_scale)? // / (1)
        .broadcast_mul(&w_anchor)?; // * (896)
                                    // = (batch_size, 896)

    // Bounding box
    let y_min = (&y_center - h.broadcast_div(&two)?)?; // (batch_size, 896)
    let x_min = (&x_center - w.broadcast_div(&two)?)?;
    let y_max = (&y_center + h.broadcast_div(&two)?)?;
    let x_max = (&x_center + w.broadcast_div(&two)?)?;

    coordinates.push(y_min);
    coordinates.push(x_min);
    coordinates.push(y_max);
    coordinates.push(x_max);

    // Face keypoints: right_eye, left_eye, nose, mouth, right_ear, left_ear
    for k in 0..6 {
        let offset = 4 + k * 2; // 4 = bounding box, 2 = (x, y)

        let keypoint_y = raw_boxes
            .i((.., .., offset))?
            .broadcast_div(&config.y_scale)?
            .broadcast_mul(&h_anchor)?
            .broadcast_add(&y_anchor)?;

        let keypoint_x = raw_boxes
            .i((.., .., offset + 1))? // (batch_size, 896)
            .broadcast_div(&config.x_scale)? // / (1)
            .broadcast_mul(&w_anchor)? // * (896)
            .broadcast_add(&x_anchor)?; // + (896)
                                        // = (batch_size, 896)

        coordinates.push(keypoint_y);
        coordinates.push(keypoint_x);
    }

    Tensor::stack(&coordinates, 2) // (batch_size, 896, 16)
}

fn unmasked_indices(
    scores: &Tensor, // (batch_size, 896, 1)
    threshold: f32,
) -> Result<Tensor> // (batch_size, num_unmasked) of DType::U32
{
    let batch_size = scores.dims()[0];

    let mask = scores
        .ge(threshold)? // (batch_size, 896, 1) of Dtype::U8
        .squeeze(2)?; // (batch_size, 896) of Dtype::U8

    // Collect unmasked indices
    let mut indices = Vec::new();
    for batch in 0..batch_size {
        let mut batch_indices = Vec::new();
        let batch_mask = mask
            .i((batch, ..))? // (896)
            .to_vec1::<u8>()?;

        batch_mask.iter().enumerate().for_each(|(i, x)| {
            if *x == 1u8 {
                batch_indices.push(i as u32);
            }
        });

        let batch_indices =
            Tensor::from_slice(&batch_indices, batch_indices.len(), scores.device())?; // (num_unmasked)

        indices.push(batch_indices);
    }

    Tensor::stack(&indices, 0) // (batch_size, num_unmasked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{safetensors, DType, Device, Tensor};
    use half::f16;

    #[test]
    fn test_forward_back() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;
        let batch_size = 1;

        // Load the variables
        let safetensors =
            safetensors::load("src/blaze_face/data/blazefaceback.safetensors", &device).unwrap();
        let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, &device);

        // Load the anchors
        let anchors = Tensor::read_npy("src/blaze_face/data/anchorsback.npy").unwrap();
        assert_eq!(anchors.dims(), &[896, 4,]);

        // Load the model
        let model = BlazeFace::load(ModelType::Back, &variables, anchors, 100., 0.65, 0.3).unwrap();

        // Set up the input Tensor
        let input = Tensor::zeros((batch_size, 3, 256, 256), dtype, &device).unwrap();

        // Call forward method and get the output
        let output = model.forward(&input).unwrap();

        assert_eq!(output.0.dims(), &[batch_size, 896, 16]);
        assert_eq!(output.1.dims(), &[batch_size, 896, 1]);
    }

    #[test]
    fn test_forward_front() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;
        let batch_size = 1;

        // Load the variables
        let safetensors =
            safetensors::load("src/blaze_face/data/blazeface.safetensors", &device).unwrap();
        let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, &device);

        // Load the anchors
        let anchors = Tensor::read_npy("src/blaze_face/data/anchors.npy").unwrap();
        assert_eq!(anchors.dims(), &[896, 4,]);

        // Load the model
        let model =
            BlazeFace::load(ModelType::Front, &variables, anchors, 100., 0.75, 0.3).unwrap();

        // Set up the input Tensor
        let input = Tensor::zeros((batch_size, 3, 128, 128), dtype, &device).unwrap();

        // Call forward method and get the output
        let output = model.forward(&input).unwrap();

        assert_eq!(output.0.dims(), &[batch_size, 896, 16]);
        assert_eq!(output.1.dims(), &[batch_size, 896, 1]);
    }

    #[test]
    fn test_decode_boxes() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;
        let batch_size = 1;

        // Set up the anchors and configuration
        let anchors = Tensor::read_npy("src/blaze_face/data/anchorsback.npy")
            .unwrap()
            .to_dtype(dtype)
            .unwrap();
        let config = BlazeFaceConfig::back(100., 0.65, 0.3, &device).unwrap();

        // Set up the input Tensor
        let input = Tensor::rand(-1., 1., (batch_size, 896, 16), &device)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Decode boxes
        let boxes = decode_boxes(&input, &anchors, &config).unwrap();

        assert_eq!(boxes.dims(), &[batch_size, 896, 16]);
    }

    #[test]
    fn test_unmasked_indices() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;
        let batch_size = 1;

        // Set up the ones Tensor
        let ones = Tensor::ones((batch_size, 896, 1), dtype, &device).unwrap();

        // Unmasked indices
        let indices = unmasked_indices(&ones, 0.5).unwrap();

        assert_eq!(indices.dims(), &[batch_size, 896]);

        // Set up the zeros Tensor
        let zeros = Tensor::zeros((batch_size, 896, 1), dtype, &device).unwrap();

        // Unmasked indices
        let indices = unmasked_indices(&zeros, 0.5).unwrap();

        assert_eq!(indices.dims(), &[batch_size, 0]);

        // Set up the test tensor
        let input = Tensor::from_slice(
            &[
                0.8, 0., 0., 0., 0., 0., 0., 0., 0., 0.4, //
                0., 0., 1., 0., 0., 0., 0., 0.7, 0., 0., //
                0., 0., 0., 0., 0., 0.8, 0., 0.1, 0.6, 0., //
            ],
            (batch_size, 30, 1),
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        // Unmasked indices
        let indices = unmasked_indices(&input, 0.5).unwrap();

        assert_eq!(indices.dims(), &[batch_size, 5]);

        assert_eq!(
            indices.squeeze(0).unwrap().to_vec1::<u32>().unwrap(),
            &[0, 12, 17, 25, 28]
        );
    }

    #[test]
    fn test_tensors_to_detections() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;
        let batch_size = 1;

        // Load the variables
        let safetensors =
            safetensors::load("src/blaze_face/data/blazefaceback.safetensors", &device).unwrap();
        let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, &device);

        // Load the anchors
        let anchors = Tensor::read_npy("src/blaze_face/data/anchorsback.npy")
            .unwrap()
            .to_dtype(dtype)
            .unwrap(); // (896, 4)
        assert_eq!(anchors.dims(), &[896, 4]);

        // Load the model
        let model = BlazeFace::load(ModelType::Back, &variables, anchors, 100., 0.65, 0.3).unwrap();

        // Set up the input Tensor
        let input = Tensor::zeros((batch_size, 3, 256, 256), dtype, &device).unwrap(); // (batch_size, 3, 256, 256)
        assert_eq!(input.dims(), &[batch_size, 3, 256, 256]);

        // Call forward method and get the output
        let (raw_boxes, raw_scores) = model.forward(&input).unwrap();
        // raw_scores: (batch_size, 896, 1), raw_boxes: (batch_size, 896, 16)
        assert_eq!(raw_boxes.dims(), &[batch_size, 896, 16]);
        assert_eq!(raw_scores.dims(), &[batch_size, 896, 1]);

        // Tensors to detections
        let detections =
            tensors_to_detections(&raw_boxes, &raw_scores, &model.anchors, &model.config).unwrap(); // Vec<(num_detections, 17)> with length:batch_size

        assert_eq!(detections.len(), batch_size);
        assert_eq!(detections[0].dims(), &[0, 17]);
    }

    #[test]
    fn test_tensors_to_detections_by_1face_front() {
        // Set up the device and dtype
        let device = Device::cuda_if_available(0).unwrap();
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Front, 0.75, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/1face.png").unwrap();
        let input = convert_image_to_tensor(&image, &device) // (3, 128, 128)
            .unwrap()
            .unsqueeze(0) // (1, 3, 128, 128)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Call forward method and get the output
        let (raw_boxes, raw_scores) = model.forward(&input).unwrap();
        // raw_boxes: (batch_size, 896, 16), raw_scores: (batch_size, 896, 1)

        // Tensors to detections
        let detections =
            tensors_to_detections(&raw_boxes, &raw_scores, &model.anchors, &model.config).unwrap(); // Vec<(num_detections, 17)> with length:batch_size

        let expected = if device.is_cpu() {
            vec![f16::from_f32(0.76187944)]
        } else {
            vec![f16::from_f32(0.7618404)]
        };

        assert_eq!(detections.len(), 1);
        assert_eq!(
            detections[0].i((.., 16)).unwrap().to_vec1::<f16>().unwrap(),
            expected
        );
    }

    #[test]
    fn test_tensors_to_detections_by_3faces_front() {
        // Set up the device and dtype
        let device = Device::cuda_if_available(0).unwrap();
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Front, 0.62, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/3faces.png").unwrap();
        let input = convert_image_to_tensor(&image, &device) // (3, 128, 128)
            .unwrap()
            .unsqueeze(0) // (1, 3, 128, 128)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Call forward method and get the output
        let (raw_boxes, raw_scores) = model.forward(&input).unwrap();
        // raw_boxes: (batch_size, 896, 16), raw_scores: (batch_size, 896, 1)

        // Tensors to detections
        let detections =
            tensors_to_detections(&raw_boxes, &raw_scores, &model.anchors, &model.config).unwrap(); // Vec<(num_detections, 17)> with length:batch_size

        let expected = if device.is_cpu() {
            vec![
                f16::from_f32(0.7212041),
                f16::from_f32(0.7330125),
                f16::from_f32(0.6364208),
            ]
        } else {
            vec![
                f16::from_f32(0.7246094),
                f16::from_f32(0.7368164),
                f16::from_f32(0.63916016),
            ]
        };

        assert_eq!(detections.len(), 1);
        assert_eq!(
            detections[0].i((.., 16)).unwrap().to_vec1::<f16>().unwrap(),
            expected
        );
    }

    #[test]
    fn test_tensors_to_detections_by_4faces_back() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Back, 0.62, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/4faces.png").unwrap().resize_exact(
            256,
            256,
            image::imageops::FilterType::Nearest,
        );
        let input = convert_image_to_tensor(&image, &device) // (3, 256, 256)
            .unwrap()
            .unsqueeze(0) // (1, 3, 256, 256)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Call forward method and get the output
        let (raw_boxes, raw_scores) = model.forward(&input).unwrap();
        // raw_boxes: (batch_size, 896, 16), raw_scores: (batch_size, 896, 1)

        // Tensors to detections
        let detections =
            tensors_to_detections(&raw_boxes, &raw_scores, &model.anchors, &model.config).unwrap(); // Vec<(num_detections, 17)> with length:batch_size

        assert_eq!(detections.len(), 1);
        assert_eq!(
            detections[0].i((.., 16)).unwrap().to_vec1::<f16>().unwrap(),
            vec![
                f16::from_f32(0.85839844),
                f16::from_f32(0.83740234),
                f16::from_f32(0.67333984),
                f16::from_f32(0.64746094)
            ]
        );
    }

    #[test]
    fn test_predict_on_batch_by_1face_front() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Front, 0.75, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/1face.png").unwrap();
        let input = convert_image_to_tensor(&image, &device) // (3, 128, 128)
            .unwrap()
            .unsqueeze(0) // (1, 3, 128, 128)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Predict on batch
        let detections = model.predict_on_batch(&input).unwrap(); // Vec<Vec<(17)>> with length:batch_size of length:num_detections

        assert_eq!(
            detections[0][0].i(16).unwrap().to_vec0::<f16>().unwrap(),
            f16::from_f32(0.76187944)
        );
    }

    #[test]
    fn test_predict_on_batch_by_1face_back() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Back, 0.62, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/1face.png").unwrap().resize_exact(
            256,
            256,
            image::imageops::FilterType::Nearest,
        );
        let input = convert_image_to_tensor(&image, &device) // (3, 256, 256)
            .unwrap()
            .unsqueeze(0) // (1, 3, 256, 256)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Predict on batch
        let detections = model.predict_on_batch(&input).unwrap(); // Vec<Vec<(17)>> with length:batch_size of length:num_detections

        assert_eq!(
            detections[0][0].i(16).unwrap().to_vec0::<f16>().unwrap(),
            f16::from_f32(0.8166175)
        );
    }

    #[test]
    fn test_predict_on_batch_by_3faces_front() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Front, 0.55, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/3faces.png").unwrap();
        let input = convert_image_to_tensor(&image, &device) // (3, 128, 128)
            .unwrap()
            .unsqueeze(0) // (1, 3, 128, 128)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Predict on batch
        let detections = model.predict_on_batch(&input).unwrap(); // Vec<Vec<(17)>> with length:batch_size of length:num_detections

        // Convert detections to scores vector
        let scores = detections[0]
            .iter()
            .map(|detection| detection.i(16).unwrap().to_vec0::<f16>().unwrap())
            .collect::<Vec<f16>>();

        assert_eq!(
            scores,
            vec![
                f16::from_f32(0.7270508),
                f16::from_f32(0.67333984),
                f16::from_f32(0.60009766)
            ]
        );
    }

    #[test]
    fn test_predict_on_batch_by_3faces_back() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Back, 0.55, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/3faces.png").unwrap().resize_exact(
            256,
            256,
            image::imageops::FilterType::Nearest,
        );
        let input = convert_image_to_tensor(&image, &device) // (3, 256, 256)
            .unwrap()
            .unsqueeze(0) // (1, 3, 256, 256)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Predict on batch
        let detections = model.predict_on_batch(&input).unwrap(); // Vec<Vec<(17)>> with length:batch_size of length:num_detections

        // Convert detections to scores vector
        let scores = detections[0]
            .iter()
            .map(|detection| detection.i(16).unwrap().to_vec0::<f16>().unwrap())
            .collect::<Vec<f16>>();

        assert_eq!(
            scores,
            vec![
                f16::from_f32(0.7521865),
                f16::from_f32(0.7521865),
                f16::from_f32(0.7521865)
            ]
        );
    }

    #[test]
    fn test_predict_on_batch_by_4faces_front() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Front, 0.6, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/4faces.png").unwrap();
        let input = convert_image_to_tensor(&image, &device) // (3, 128, 128)
            .unwrap()
            .unsqueeze(0) // (1, 3, 128, 128)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Predict on batch
        let detections = model.predict_on_batch(&input).unwrap(); // Vec<Vec<(17)>> with length:batch_size of length:num_detections

        // Convert detections to scores vector
        let scores = detections[0]
            .iter()
            .map(|detection| detection.i(16).unwrap().to_vec0::<f16>().unwrap())
            .collect::<Vec<f16>>();

        assert_eq!(scores, Vec::<f16>::new());
    }

    #[test]
    fn test_predict_on_batch_by_4faces_back() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Load the model
        let model = load_model(ModelType::Back, 0.5, &device, dtype).unwrap();

        // Load the test image
        let image = image::open("test_data/4faces.png").unwrap().resize_exact(
            256,
            256,
            image::imageops::FilterType::Nearest,
        );
        let input = convert_image_to_tensor(&image, &device) // (3, 256, 256)
            .unwrap()
            .unsqueeze(0) // (1, 3, 256, 256)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Predict on batch
        let detections = model.predict_on_batch(&input).unwrap(); // Vec<Vec<(17)>> with length:batch_size of length:num_detections

        // Convert detections to scores vector
        let scores = detections[0]
            .iter()
            .map(|detection| detection.i(16).unwrap().to_vec0::<f16>().unwrap())
            .collect::<Vec<f16>>();

        assert_eq!(
            scores,
            vec![f16::from_f32(0.84765625), f16::from_f32(0.7109375)]
        );
    }

    #[test]
    fn test_color_order() {
        let device = Device::Cpu;

        let colors = vec![
            0.1, 0.2, 0.3, // (0, 0)
            0.4, 0.5, 0.6, // (1, 0)
            0.7, 0.8, 0.9, // (0, 1)
            0.11, 0.12, 0.13, // (1, 1)
        ];

        let tensor = Tensor::from_vec(colors, (2, 2, 3), &device).unwrap();

        assert_eq!(
            tensor.to_vec3::<f64>().unwrap(),
            vec![
                vec![vec![0.1, 0.2, 0.3], vec![0.4, 0.5, 0.6],],
                vec![vec![0.7, 0.8, 0.9], vec![0.11, 0.12, 0.13],],
            ]
        );

        let tensor = tensor.permute((2, 0, 1)).unwrap();

        assert_eq!(
            tensor.to_vec3::<f64>().unwrap(),
            vec![
                vec![
                    // R
                    vec![
                        // W = 0
                        0.1, // H = 0
                        0.4  // H = 1
                    ],
                    vec![
                        // W = 1
                        0.7,  // H = 0
                        0.11  // H = 1
                    ],
                ],
                vec![
                    // G
                    vec![0.2, 0.5],
                    vec![0.8, 0.12],
                ],
                vec![
                    // G
                    vec![0.3, 0.6],
                    vec![0.9, 0.13],
                ],
            ]
        );

        let tensor = tensor.permute((0, 2, 1)).unwrap();

        assert_eq!(
            tensor.to_vec3::<f64>().unwrap(),
            vec![
                vec![
                    // R
                    vec![
                        // H = 0
                        0.1, // W = 0
                        0.7  // W = 1
                    ],
                    vec![
                        // H = 1
                        0.4,  // W = 0
                        0.11  // W = 1
                    ],
                ],
                vec![
                    // G
                    vec![
                        // H = 0
                        0.2, // W = 0
                        0.8  // W = 1
                    ],
                    vec![
                        // H = 1
                        0.5,  // W = 0
                        0.12  // W = 1
                    ],
                ],
                vec![
                    // B
                    vec![
                        // H = 0
                        0.3, // W = 0
                        0.9  // W = 1
                    ],
                    vec![
                        // H = 1
                        0.6,  // W = 0
                        0.13  // W = 1
                    ],
                ],
            ]
        );
    }

    fn load_model(
        model_type: ModelType,
        min_score_threshold: f32,
        device: &Device,
        dtype: DType,
    ) -> Result<BlazeFace> {
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
            0.3,
        )
    }

    fn convert_image_to_tensor(image: &image::DynamicImage, device: &Device) -> Result<Tensor> {
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
}
