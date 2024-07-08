// Reference implementation:
// https://github.com/hollance/BlazeFace-PyTorch/blob/master/blazeface.py

use candle_core::{Error, IndexOp, Result, Shape, Tensor};
use half::f16;

use super::blaze_face_config::DTYPE_IN_BLAZE_FACE;

pub(crate) fn weighted_non_max_suppression(
    detections: &Tensor, // (num_detections, 17)
    min_suppression_threshold: f32,
) -> Result<Vec<Tensor>> // Vector of weighted detections by non-maximum suppression
{
    if detections.dims()[0] == 0 {
        return Ok(Vec::new());
    }
    if detections.dims()[1] != 17 {
        return Err(Error::ShapeMismatchBinaryOp {
            lhs: detections.shape().clone(),
            rhs: Shape::from_dims(&[detections.dims()[0], 17]),
            op: "weighted_non_max_suppression",
        });
    }

    let mut output = Vec::new();

    // Sort by score
    let mut remaining = argsort_by_score(detections)?; // (num_detections) of Dtype::U32

    while remaining.dims()[0] > 0 {
        // Highest score detection
        let detection = detections.i((remaining.to_vec1::<u32>()?[0] as usize, ..))?; // (17)

        // Highest score box
        let first_box = detection.i(0..4)?; // (4)
                                            // Remaining boxes including the first box
        let other_box = detections.i((&remaining, ..4))?; // (remainings, 4)

        // NOTE: ious = IoUs (Intersection over Unions)
        let ious = overlap_similarity(&first_box, &other_box)?; // (remainings)
        let mask = ious.gt(min_suppression_threshold)?; // (remainings) of Dtype::U8
        let (overlapping, others) = mask_indices(&mask)?; // (unmasked_indices), (masked_indices)

        remaining = others; // (unmasked_indices) of Dtype::U32

        let mut weighted_detection = detection.clone(); // (17)

        if overlapping.dims()[0] > 1 {
            let overlapped = detections.i((&overlapping, ..))?; // (overlapped, 17)
            let coordinates = overlapped.i((.., 0..16))?; // (overlapped, 16)
            let scores = overlapped
                .i((.., 16))? // (overlapped)
                .unsqueeze(1)?; // (overlapped, 1)
            let total_score = scores.sum(0)?; // (1)
            let overlapped_count =
                Tensor::from_slice(&[overlapping.dims()[0] as f32], 1, detections.device())?
                    .to_dtype(DTYPE_IN_BLAZE_FACE)?; // (1)

            let weighted_coordinates = coordinates
                .broadcast_mul(&scores)? // (overlapped, 16)
                .sum(0)? // (16)
                .broadcast_div(&total_score)?; // (16)

            let weighted_score = total_score.div(&overlapped_count)?; // (1)

            weighted_detection = Tensor::cat(&[weighted_coordinates, weighted_score], 0)?;
            // (17)
        }

        output.push(weighted_detection);
    }

    Ok(output)
}

fn argsort_by_score(detection: &Tensor, // (num_detections, 17)
) -> Result<Tensor> // (num_detections) of DType::U32
{
    let scores = detection
        .i((.., 16))? // (num_detections)
        .to_vec1::<f16>()?;

    let count = scores.len();

    // Create a vector of indices from 0 to num_detections - 1
    let mut indices: Vec<u32> = (0u32..count as u32).collect();

    // Sort the indices by descending order of scores
    indices.sort_unstable_by(|&a, &b| {
        let score_a = scores[a as usize];
        let score_b = scores[b as usize];

        // Reverse
        score_b.partial_cmp(&score_a).unwrap()
    });

    Tensor::from_vec(indices, count, detection.device())
}

fn overlap_similarity(
    first_box: &Tensor, // (4)
    other_box: &Tensor, // (remainings, 4)
) -> Result<Tensor> // (remainings)
{
    let first_box = first_box.unsqueeze(0)?; // (1, 4)

    jaccard(&first_box, other_box)? // (1, remainings)
        .squeeze(0) // (remainings)
}

fn jaccard(
    box_a: &Tensor, // (a, 4)
    box_b: &Tensor, // (b, 4)
) -> Result<Tensor> // (a, b)
{
    let inter = intersect(box_a, box_b)?; // (a, b)

    let area_a = box_a
        .i((.., 2))?
        .sub(&box_a.i((.., 0))?)?
        .mul(&box_a.i((.., 3))?.sub(&box_a.i((.., 1))?)?)?
        .unsqueeze(1)?
        .expand(inter.shape())?; // (a, b)

    let area_b = box_b
        .i((.., 2))?
        .sub(&box_b.i((.., 0))?)?
        .mul(&box_b.i((.., 3))?.sub(&box_b.i((.., 1))?)?)?
        .unsqueeze(0)?
        .expand(inter.shape())?; // (a, b)

    let union = ((&area_a + &area_b)? - &inter)?; // (a, b)

    inter.div(&union) // (a, b)
}

fn intersect(
    box_a: &Tensor, // (a, 4)
    box_b: &Tensor, // (b, 4)
) -> Result<Tensor> // (a, b)
{
    let a = box_a.dims()[0];
    let b = box_b.dims()[0];

    let a_max_xy = box_a.i((.., 2..4))?.unsqueeze(1)?.expand(&[a, b, 2])?; // (a, b, 2)

    let b_max_xy = box_b.i((.., 2..4))?.unsqueeze(0)?.expand(&[a, b, 2])?; // (a, b, 2)

    let a_min_xy = box_a.i((.., 0..2))?.unsqueeze(1)?.expand(&[a, b, 2])?; // (a, b, 2)

    let b_min_xy = box_b.i((.., 0..2))?.unsqueeze(0)?.expand(&[a, b, 2])?; // (a, b, 2)

    let max_xy = Tensor::stack(&[a_max_xy, b_max_xy], 0)?.min(0)?; // (a, b, 2)
    let min_xy = Tensor::stack(&[a_min_xy, b_min_xy], 0)?.max(0)?; // (a, b, 2)
    let inter = Tensor::clamp(&(max_xy - min_xy)?, 0., f16::INFINITY)?; // (a, b, 2)

    inter.i((.., .., 0))?.mul(&inter.i((.., .., 1))?) // (a, b)
}

fn mask_indices(mask: &Tensor, // (masked_vector) of DType::U8
) -> Result<(Tensor, Tensor)> // (unmasked_indices), (masked_indices) of DType::U32
{
    let mut unmasked = Vec::new();
    let mut masked = Vec::new();
    for (i, x) in mask.to_vec1::<u8>()?.iter().enumerate() {
        if *x == 1u8 {
            unmasked.push(i as u32);
        } else {
            masked.push(i as u32);
        }
    }

    let unmasked = Tensor::from_slice(&unmasked, unmasked.len(), mask.device())?;

    let masked = Tensor::from_slice(&masked, masked.len(), mask.device())?;

    Ok((unmasked, masked))
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{DType, Device, Tensor};

    #[test]
    fn test_argsort() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Set up the input Tensor
        let right_eye = Tensor::from_slice(
            &[
                0.8, 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.4,
            ],
            17,
            &device,
        )
        .unwrap(); // (17)
        let left_eye = Tensor::from_slice(
            &[
                0., 0.7, 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0.8,
            ],
            17,
            &device,
        )
        .unwrap(); // (17)
        let input = Tensor::stack(&[right_eye, left_eye], 0)
            .unwrap()
            .to_dtype(dtype)
            .unwrap(); // (2, 17)
        assert_eq!(input.dims(), &[2, 17]);
        assert_eq!(
            input.i((0, 16)).unwrap().to_vec0::<f16>().unwrap(),
            f16::from_f32(0.4),
        );
        assert_eq!(
            input.i((1, 16)).unwrap().to_vec0::<f16>().unwrap(),
            f16::from_f32(0.8),
        );

        // Sort
        let sorted = argsort_by_score(&input).unwrap();
        assert_eq!(sorted.dims()[0], 2);
        assert_eq!(sorted.to_vec1::<u32>().unwrap()[0], 1);
        assert_eq!(sorted.to_vec1::<u32>().unwrap()[1], 0);
    }

    #[test]
    fn test_intersect() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Set up the boxes Tensors
        let box_a = Tensor::from_slice(
            &[
                0., 0., 10., 10., //
                10., 10., 20., 20., //
            ],
            (2, 4),
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();
        assert_eq!(box_a.dims(), &[2, 4]);
        assert_eq!(
            box_a.to_vec2::<f16>().unwrap(),
            vec![
                [
                    f16::from_f32(0.),
                    f16::from_f32(0.),
                    f16::from_f32(10.),
                    f16::from_f32(10.),
                ], //
                [
                    f16::from_f32(10.),
                    f16::from_f32(10.),
                    f16::from_f32(20.),
                    f16::from_f32(20.),
                ], //
            ],
        );

        let box_b = Tensor::from_slice(
            &[
                5., 5., 15., 15., //
                15., 15., 25., 25., //
            ],
            (2, 4),
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        // Intersect
        let intersect = intersect(&box_a, &box_b).unwrap(); // (2, 2)

        assert_eq!(intersect.dims(), &[2, 2]);
        assert_eq!(
            intersect.to_vec2::<f16>().unwrap(),
            vec![
                [
                    f16::from_f32(25.), // (0, 0, 10, 10) intersects (5, 5, 15, 15) with area 25
                    f16::from_f32(0.),  // (0, 0, 10, 10) does not intersect (15, 15, 25, 25)
                ],
                [
                    f16::from_f32(25.), // (10, 10, 20, 20) intersects (5, 5, 15, 15) with area 25
                    f16::from_f32(25.), // (10, 10, 20, 20) intersects (15, 15, 25, 25) with area 25
                ],
            ]
        );
    }

    #[test]
    fn test_jaccard() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Set up the boxes Tensors
        let box_a = Tensor::from_slice(
            &[
                0., 0., 10., 10., //
                10., 10., 20., 20., //
            ],
            (2, 4),
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        let box_b = Tensor::from_slice(
            &[
                5., 5., 15., 15., //
                15., 15., 25., 25., //
            ],
            (2, 4),
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        // Jaccard
        let jaccard = jaccard(&box_a, &box_b).unwrap(); // (2, 2)

        assert_eq!(jaccard.dims(), &[2, 2]);
        assert_eq!(
            jaccard.to_vec2::<f16>().unwrap(),
            vec![
                [
                    f16::from_f32(1. / 7.), // = 25 / (100 + 100 - 25)
                    f16::from_f32(0.),      // = 0 / (100 + 100 - 0)
                ],
                [
                    f16::from_f32(1. / 7.), // = 25 / (100 + 100 - 25)
                    f16::from_f32(1. / 7.), // = 25 / (100 + 100 - 25)
                ],
            ]
        );
    }

    #[test]
    fn test_overlap_similarity() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Set up the boxes Tensors
        let box_a = Tensor::from_slice(
            &[
                0., 0., 10., 10., //
            ],
            4,
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        let box_b = Tensor::from_slice(
            &[
                5., 5., 15., 15., //
                15., 15., 25., 25., //
            ],
            (2, 4),
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        // Overlap similarity
        let similarity = overlap_similarity(&box_a, &box_b).unwrap(); // (2)

        assert_eq!(
            similarity.to_vec1::<f16>().unwrap(),
            vec![
                f16::from_f32(1. / 7.), // = 25 / (100 + 100 - 25)
                f16::from_f32(0.),      // = 0  / (100 + 100 - 25)
            ]
        );

        let box_c = Tensor::from_slice(
            &[
                0., 0., 10., 10., //
            ],
            (1, 4),
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap();

        let same_similarity = overlap_similarity(&box_a, &box_c).unwrap(); // (1)
        assert_eq!(
            same_similarity.to_vec1::<f16>().unwrap(),
            vec![
                f16::from_f32(1.), // = 100 / (100 + 100 - 100)
            ]
        );
    }

    #[test]
    fn test_tensor_mask() {
        let device = Device::Cpu;

        let tensor = Tensor::from_slice(
            &[0., 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.],
            11,
            &device,
        )
        .unwrap();

        let threashold = 0.4;

        let mask = tensor.gt(threashold).unwrap();
        assert_eq!(
            mask.to_vec1::<u8>().unwrap(),
            vec![0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1]
        );

        let unmasked = tensor.i(&mask).unwrap();
        assert_eq!(
            unmasked.to_vec1::<f64>().unwrap(),
            vec![0., 0., 0., 0., 0., 0.1, 0.1, 0.1, 0.1, 0.1, 0.1]
        );

        let unmask = tensor.le(threashold).unwrap();
        assert_eq!(
            unmask.to_vec1::<u8>().unwrap(),
            vec![1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0]
        );

        let masked = tensor.i(&unmask).unwrap();
        assert_eq!(
            masked.to_vec1::<f64>().unwrap(),
            vec![0.1, 0.1, 0.1, 0.1, 0.1, 0., 0., 0., 0., 0., 0.]
        );

        let selected = tensor.index_select(&mask, 0).unwrap();
        assert_eq!(
            selected.to_vec1::<f64>().unwrap(),
            vec![0., 0., 0., 0., 0., 0.1, 0.1, 0.1, 0.1, 0.1, 0.1] // equals to unmasked
        );
    }

    #[test]
    fn test_mask_indices() {
        let device = Device::Cpu;
        let dtype = DType::F16;

        let similarities = Tensor::from_slice(&[0., 0.1, 0.2, 0.3, 0.4, 0.5], 6, &device)
            .unwrap()
            .to_dtype(dtype)
            .unwrap(); // (6)

        let threashold = 0.3;

        let (unmasked, masked) = mask_indices(&similarities.gt(threashold).unwrap()).unwrap(); // (2), (4)

        assert_eq!(unmasked.to_vec1::<u32>().unwrap(), vec![4, 5]);
        assert_eq!(masked.to_vec1::<u32>().unwrap(), vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_weighted_non_max_suppression() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Setup the detections
        let detection_1 = Tensor::from_slice(
            &[
                0.1, 0.1, 0.2, 0.2, // Bounding box
                0.0, 0.0, // Right eye
                0.0, 0.0, // Left eye
                0.0, 0.0, // Nose
                0.0, 0.0, // Mouth
                0.0, 0.0, // Right ear
                0.0, 0.0, // Left ear
                0.9, // Score
            ],
            17,
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap(); // (17)

        let detection_2 = Tensor::from_slice(
            &[
                0.11, 0.11, 0.22, 0.22, // Bounding box
                0.0, 0.0, // Right eye
                0.0, 0.0, // Left eye
                0.0, 0.0, // Nose
                0.0, 0.0, // Mouth
                0.0, 0.0, // Right ear
                0.0, 0.0, // Left ear
                0.8, // Score
            ],
            17,
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap(); // (17)

        let detection_3 = Tensor::from_slice(
            &[
                0.09, 0.09, 0.19, 0.19, // Bounding box
                0.0, 0.0, // Right eye
                0.0, 0.0, // Left eye
                0.0, 0.0, // Nose
                0.0, 0.0, // Mouth
                0.0, 0.0, // Right ear
                0.0, 0.0, // Left ear
                0.7, // Score
            ],
            17,
            &device,
        )
        .unwrap()
        .to_dtype(dtype)
        .unwrap(); // (17)

        let detections = Tensor::stack(&[detection_1, detection_2, detection_3], 0).unwrap(); // (3, 17)

        // Calculate weighted non-maximum suppression
        let weighted_detections = weighted_non_max_suppression(&detections, 0.3).unwrap(); // Vec<(num_detections, 17)> with length:batch_size

        assert_eq!(weighted_detections.len(), 1);
        assert_eq!(
            weighted_detections[0].to_vec1::<f16>().unwrap(),
            vec![
                f16::from_f32(0.10046387),
                f16::from_f32(0.10046387),
                f16::from_f32(0.20385742),
                f16::from_f32(0.20385742), // > (0.1, 0.1, 0.2, 0.2)
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.),
                f16::from_f32(0.7993164)
            ] // ~ 0.8
        );
    }
}
