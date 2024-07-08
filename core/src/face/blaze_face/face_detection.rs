use candle_core::{Error, Result, Shape, Tensor};
use half::f16;

#[derive(Debug)]
pub struct BoundingBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

#[derive(Debug)]
pub struct Keypoints {
    pub right_eye: KeyPoint,
    pub left_eye: KeyPoint,
    pub nose: KeyPoint,
    pub mouth: KeyPoint,
    pub right_ear: KeyPoint,
    pub left_ear: KeyPoint,
}

#[derive(Debug)]
pub struct KeyPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug)]
pub struct FaceDetection {
    pub bounding_box: BoundingBox,
    pub key_points: Keypoints,
    pub score: f32,
}

impl FaceDetection {
    pub fn from_tensor(tensor: &Tensor, // (17)
    ) -> Result<Self> {
        if tensor.dims() != [17] {
            return Result::Err(Error::ShapeMismatchBinaryOp {
                lhs: tensor.shape().clone(),
                rhs: Shape::from(&[17]),
                op: "from_tensor",
            });
        }

        let vector = tensor.to_vec1::<f16>()?;

        let bounding_box = BoundingBox {
            x_min: vector[1].to_f32(),
            y_min: vector[0].to_f32(),
            x_max: vector[3].to_f32(),
            y_max: vector[2].to_f32(),
        };

        let key_points = Keypoints {
            right_eye: KeyPoint {
                x: vector[5].to_f32(),
                y: vector[4].to_f32(),
            },
            left_eye: KeyPoint {
                x: vector[7].to_f32(),
                y: vector[6].to_f32(),
            },
            nose: KeyPoint {
                x: vector[9].to_f32(),
                y: vector[8].to_f32(),
            },
            mouth: KeyPoint {
                x: vector[11].to_f32(),
                y: vector[10].to_f32(),
            },
            right_ear: KeyPoint {
                x: vector[13].to_f32(),
                y: vector[12].to_f32(),
            },
            left_ear: KeyPoint {
                x: vector[15].to_f32(),
                y: vector[14].to_f32(),
            },
        };

        let score = vector[16].to_f32();

        Ok(Self {
            bounding_box,
            key_points,
            score,
        })
    }

    pub fn from_tensors(tensors: Vec<Tensor>, // Vec<(17)>
    ) -> Result<Vec<Self>> {
        let mut face_detections = Vec::with_capacity(tensors.len());

        for tensor in tensors {
            face_detections.push(Self::from_tensor(&tensor)?);
        }

        Ok(face_detections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device, Tensor};

    #[test]
    fn test_from_tensor() {
        let device = Device::Cpu;
        let dtype = DType::F16;

        let tensor = Tensor::from_slice(
            &[
                0.1, 0.11, 0.9, 0.91, // bounding box
                0.7, 0.6, // right eye
                0.3, 0.6, // left eye
                0.5, 0.5, // nose
                0.5, 0.3, // mouth
                0.8, 0.55, // right ear
                0.2, 0.55, // left ear
                0.9,  // score
            ],
            17,
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        let _face_detection = FaceDetection::from_tensor(&tensor).unwrap();
    }

    #[test]
    fn test_from_tensors() {
        let device = Device::Cpu;
        let dtype = DType::F16;

        let tensor_1 = Tensor::from_slice(
            &[
                0.1, 0.11, 0.9, 0.91, // bounding box
                0.7, 0.6, // right eye
                0.3, 0.6, // left eye
                0.5, 0.5, // nose
                0.5, 0.3, // mouth
                0.8, 0.55, // right ear
                0.2, 0.55, // left ear
                0.9,  // score
            ],
            17,
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        let tensor_2 = Tensor::from_slice(
            &[
                0.2, 0.21, 0.8, 0.81, // bounding box
                0.6, 0.5, // right eye
                0.4, 0.5, // left eye
                0.6, 0.4, // nose
                0.6, 0.2, // mouth
                0.7, 0.45, // right ear
                0.3, 0.45, // left ear
                0.8,  // score
            ],
            17,
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        let face_detections = FaceDetection::from_tensors(vec![tensor_1, tensor_2]).unwrap();

        assert_eq!(face_detections.len(), 2);
    }
}
