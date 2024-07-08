// SPDX-FileCopyrightText: © 2023 Mochineko <t.o.e.4315@gmail.com>
//
// SPDX-License-Identifier: MIT
//
// Reference implementation:
// https://github.com/hollance/BlazeFace-PyTorch/blob/master/blazeface.py

use candle_core::{DType, Device, Result, Tensor};

pub(crate) const DTYPE_IN_BLAZE_FACE: DType = DType::F16;

pub struct BlazeFaceConfig {
    pub(crate) x_scale: Tensor,
    pub(crate) y_scale: Tensor,
    pub(crate) h_scale: Tensor,
    pub(crate) w_scale: Tensor,
    pub(crate) score_clipping_threshold: f32,
    pub(crate) min_score_threshold: f32,
    pub(crate) min_suppression_threshold: f32,
}

impl BlazeFaceConfig {
    pub(crate) fn back(
        score_clipping_threshold: f32,
        min_score_threshold: f32,
        min_suppression_threshold: f32,
        device: &Device,
    ) -> Result<Self> {
        Ok(Self {
            x_scale: Tensor::from_slice(&[256.], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            y_scale: Tensor::from_slice(&[256.], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            h_scale: Tensor::from_slice(&[256.], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            w_scale: Tensor::from_slice(&[256.], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            score_clipping_threshold,
            min_score_threshold,
            min_suppression_threshold,
        })
    }

    pub(crate) fn front(
        score_clipping_thresh: f32,
        min_score_thresh: f32,
        min_suppression_threshold: f32,
        device: &Device,
    ) -> Result<Self> {
        Ok(Self {
            x_scale: Tensor::from_slice(&[128_f32], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            y_scale: Tensor::from_slice(&[128_f32], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            h_scale: Tensor::from_slice(&[128_f32], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            w_scale: Tensor::from_slice(&[128_f32], 1, device)?.to_dtype(DTYPE_IN_BLAZE_FACE)?, // (1)
            score_clipping_threshold: score_clipping_thresh,
            min_score_threshold: min_score_thresh,
            min_suppression_threshold,
        })
    }
}
