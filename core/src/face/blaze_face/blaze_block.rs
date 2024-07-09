// SPDX-FileCopyrightText: © 2023 Mochineko <t.o.e.4315@gmail.com>
//
// SPDX-License-Identifier: MIT
//
// Reference implementation:
// https://github.com/hollance/BlazeFace-PyTorch/blob/master/blazeface.py

use candle_core::{Module, Result, Tensor};
use candle_nn::{Conv2d, Conv2dConfig, VarBuilder};

/// BlazeBlock backbone.
pub struct BlazeBlock {
    block_type: BlazeBlockType,
}

/// Stride type for BlazeBlock.
pub enum StrideType {
    Single,
    Double,
}

/// BlazeBlock type.
enum BlazeBlockType {
    SingleStride(BlazeBlockSingleStride),
    DoubleStride(BlazeBlockDoubleStride),
}

impl BlazeBlock {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn load(
        in_channels: usize,
        out_channels: usize,
        stride: StrideType,
        variables: &VarBuilder,
        weight_0_name: &str,
        bias_0_name: &str,
        weight_1_name: &str,
        bias_1_name: &str,
    ) -> Result<Self> {
        let weight_0 = variables.get_with_hints(
            (in_channels, 1, 3, 3),
            weight_0_name,
            candle_nn::init::ZERO,
        )?;
        let bias_0 = variables.get_with_hints(in_channels, bias_0_name, candle_nn::init::ZERO)?;
        let weight_1 = variables.get_with_hints(
            (out_channels, in_channels, 1, 1),
            weight_1_name,
            candle_nn::init::ZERO,
        )?;
        let bias_1 = variables.get_with_hints(out_channels, bias_1_name, candle_nn::init::ZERO)?;

        Self::new(
            in_channels,
            out_channels,
            3,
            stride,
            weight_0,
            bias_0,
            weight_1,
            bias_1,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        in_channels: usize,
        out_channels: usize,
        kernel_size: usize,
        stride: StrideType,
        weight_0: Tensor,
        bias_0: Tensor,
        weight_1: Tensor,
        bias_1: Tensor,
    ) -> Result<Self> {
        let block_type = match stride {
            StrideType::Single => BlazeBlockType::SingleStride(BlazeBlockSingleStride::new(
                in_channels,
                out_channels,
                kernel_size,
                weight_0,
                bias_0,
                weight_1,
                bias_1,
            )?),
            StrideType::Double => BlazeBlockType::DoubleStride(BlazeBlockDoubleStride::new(
                in_channels,
                out_channels,
                weight_0,
                bias_0,
                weight_1,
                bias_1,
            )?),
        };

        Ok(Self { block_type })
    }
}

impl Module for BlazeBlock {
    fn forward(&self, input: &Tensor) -> Result<Tensor> {
        match &self.block_type {
            BlazeBlockType::SingleStride(block) => block.forward(input),
            BlazeBlockType::DoubleStride(block) => block.forward(input),
        }
    }
}

/// BlazeBlock for stride 1.
struct BlazeBlockSingleStride {
    channel_pad: usize,
    conv0: Conv2d,
    conv1: Conv2d,
}

impl BlazeBlockSingleStride {
    fn new(
        in_channels: usize,
        out_channels: usize,
        kernel_size: usize,
        weight_0: Tensor,
        bias_0: Tensor,
        weight_1: Tensor,
        bias_1: Tensor,
    ) -> Result<Self> {
        Ok(Self {
            channel_pad: out_channels - in_channels,
            conv0: Conv2d::new(
                weight_0,
                Some(bias_0),
                Conv2dConfig {
                    padding: (kernel_size - 1) / 2,
                    groups: in_channels,
                    ..Default::default()
                },
            ),
            conv1: Conv2d::new(
                weight_1,
                Some(bias_1),
                Conv2dConfig {
                    ..Default::default()
                },
            ),
        })
    }

    fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let h = input;

        let x = if self.channel_pad != 0 {
            input.pad_with_zeros(1, 0, self.channel_pad)? // channel padding
        } else {
            input.clone()
        };

        let h = self.conv0.forward(h)?;
        let h = self.conv1.forward(&h)?;
        (h + x)?.relu()
    }
}

/// BlazeBlock for stride 2.
struct BlazeBlockDoubleStride {
    channel_pad: usize,
    conv0: Conv2d,
    conv1: Conv2d,
}

impl BlazeBlockDoubleStride {
    fn new(
        in_channels: usize,
        out_channels: usize,
        weight_0: Tensor,
        bias_0: Tensor,
        weight_1: Tensor,
        bias_1: Tensor,
    ) -> Result<Self> {
        Ok(BlazeBlockDoubleStride {
            channel_pad: out_channels - in_channels,
            conv0: Conv2d::new(
                weight_0,
                Some(bias_0),
                Conv2dConfig {
                    padding: 0,
                    stride: 2,
                    groups: in_channels,
                    ..Default::default()
                },
            ),
            conv1: Conv2d::new(
                weight_1,
                Some(bias_1),
                Conv2dConfig {
                    ..Default::default()
                },
            ),
        })
    }

    fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let h = input
            .pad_with_zeros(2, 0, 2)? // height padding
            .pad_with_zeros(3, 0, 2)?; // width padding

        let x = input.max_pool2d_with_stride(2, 2)?; // max pooling

        let x = if self.channel_pad > 0 {
            x.pad_with_zeros(1, 0, self.channel_pad)? // channel padding
        } else {
            x
        };

        let h = self.conv0.forward(&h)?;
        let h = self.conv1.forward(&h)?;
        (h + x)?.relu()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{safetensors, DType, Device, Tensor};

    #[test]
    fn test_blaze_block_for_single_stride() {
        // Set up the device
        let device = Device::Cpu;

        // Set up the configuration
        let batch_size = 1;
        let in_channels = 24;
        let out_channels = 24;
        let width = 64;
        let height = 64;
        let kernel_size = 3;

        let weight_0 =
            Tensor::rand(0., 1., (in_channels, 1, kernel_size, kernel_size), &device).unwrap();
        let bias_0 = Tensor::rand(0., 1., (in_channels,), &device).unwrap();

        let weight_1 = Tensor::rand(0., 1., (out_channels, in_channels, 1, 1), &device).unwrap();
        let bias_1 = Tensor::rand(0., 1., (out_channels,), &device).unwrap();

        // Instantiate the BlazeBlock
        let model = BlazeBlockSingleStride::new(
            in_channels,
            out_channels,
            kernel_size,
            weight_0,
            bias_0,
            weight_1,
            bias_1,
        )
        .unwrap();

        // Set up the input tensor
        let input =
            Tensor::rand(0., 1., &[batch_size, in_channels, height, width], &device).unwrap(); // (1, 24, 64, 64)

        // Call forward method and get the output
        let output = model.forward(&input).unwrap(); // (1, 24, 64, 64)
        assert_eq!(output.dims(), &[batch_size, out_channels, height, width,]);
    }

    #[test]
    fn test_blaze_block_for_single_stride_with_channel_padding() {
        // Set up the device
        let device = Device::Cpu;

        // Set up the configuration
        let batch_size = 1;
        let in_channels = 24;
        let out_channels = 28;
        let width = 64;
        let height = 64;
        let kernel_size = 3;

        let weight_0 =
            Tensor::rand(0., 1., (in_channels, 1, kernel_size, kernel_size), &device).unwrap();
        let bias_0 = Tensor::rand(0., 1., (in_channels,), &device).unwrap();

        let weight_1 = Tensor::rand(0., 1., (out_channels, in_channels, 1, 1), &device).unwrap();
        let bias_1 = Tensor::rand(0., 1., (out_channels,), &device).unwrap();

        // Instantiate the BlazeBlock
        let model = BlazeBlockSingleStride::new(
            in_channels,
            out_channels,
            kernel_size,
            weight_0,
            bias_0,
            weight_1,
            bias_1,
        )
        .unwrap();

        // Set up the input tensor
        let input =
            Tensor::rand(0., 1., &[batch_size, in_channels, height, width], &device).unwrap(); // (1, 24, 64, 64)

        // Call forward method and get the output
        let output = model.forward(&input).unwrap(); // (1, 28, 64, 64)
        assert_eq!(output.dims(), &[batch_size, out_channels, height, width,]);
    }

    #[test]
    fn test_blaze_block_for_double_stride() {
        // Set up the device
        let device = Device::Cpu;

        // Set up the configuration
        let batch_size = 1;
        let in_channels = 24;
        let out_channels = 24;
        let width = 64;
        let height = 64;
        let kernel_size = 3;

        let weight_0 =
            Tensor::rand(0., 1., (in_channels, 1, kernel_size, kernel_size), &device).unwrap();
        let bias_0 = Tensor::rand(0., 1., (in_channels,), &device).unwrap();

        let weight_1 = Tensor::rand(0., 1., (out_channels, in_channels, 1, 1), &device).unwrap();
        let bias_1 = Tensor::rand(0., 1., (out_channels,), &device).unwrap();

        // Instantiate the BlazeBlock
        let block = BlazeBlockDoubleStride::new(
            in_channels,
            out_channels,
            weight_0,
            bias_0,
            weight_1,
            bias_1,
        )
        .unwrap();

        // Set up the input tensor
        let input =
            Tensor::rand(0., 1., (batch_size, in_channels, height, width), &device).unwrap(); // (1, 24, 64, 64)

        // Call forward method and get the output
        let output = block.forward(&input).unwrap(); // (1, 24, 32, 32)
        assert_eq!(
            output.dims(),
            &[
                batch_size,
                out_channels,
                height / 2, // stride = 2
                width / 2,  // stride = 2
            ]
        );
    }

    #[test]
    fn test_blaze_block_for_double_stride_with_channel_padding() {
        // Set up the device
        let device = Device::Cpu;

        // Set up the configuration
        let batch_size = 1;
        let in_channels = 24;
        let out_channels = 28;
        let width = 64;
        let height = 64;
        let kernel_size = 3;

        let weight_0 =
            Tensor::rand(0., 1., (in_channels, 1, kernel_size, kernel_size), &device).unwrap();
        let bias_0 = Tensor::rand(0., 1., (in_channels,), &device).unwrap();

        let weight_1 = Tensor::rand(0., 1., (out_channels, in_channels, 1, 1), &device).unwrap();
        let bias_1 = Tensor::rand(0., 1., (out_channels,), &device).unwrap();

        // Instantiate the BlazeBlock
        let block = BlazeBlockDoubleStride::new(
            in_channels,
            out_channels,
            weight_0,
            bias_0,
            weight_1,
            bias_1,
        )
        .unwrap();

        // Set up the input tensor
        let input =
            Tensor::rand(0., 1., (batch_size, in_channels, height, width), &device).unwrap(); // (1, 24, 64, 64)

        // Call forward method and get the output
        let output = block.forward(&input).unwrap(); // (1, 28, 32, 32)
        assert_eq!(
            output.dims(),
            &[
                batch_size,
                out_channels,
                height / 2, // stride = 2
                width / 2,  // stride = 2
            ]
        );
    }

    #[test]
    fn test_load() {
        // Set up the device
        let device = Device::Cpu;
        let dtype = DType::F16;

        // Set up the configuration
        let batch_size = 1;
        let width = 64;
        let height = 64;

        // Load the variables
        let safetensors =
            safetensors::load("src/face/blaze_face/data/blazeface.safetensors", &device).unwrap();
        let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, &device);

        // Set up the input tensor
        let input = Tensor::rand(0., 1., &[batch_size, 24, height, width], &device)
            .unwrap()
            .to_dtype(dtype)
            .unwrap(); // (1, 24, 64, 64)

        // Instantiate the BlazeBlock for single stride
        let single_block = BlazeBlock::load(
            24,
            24,
            StrideType::Single,
            &variables,
            "backbone1.2.convs.0.weight",
            "backbone1.2.convs.0.bias",
            "backbone1.2.convs.1.weight",
            "backbone1.2.convs.1.bias",
        )
        .unwrap();

        // Call forward method and get the output
        let output = single_block.forward(&input).unwrap(); // (1, 24, 64, 64)
        assert_eq!(output.dims(), &[batch_size, 24, height, width,]);

        // Instantiate the BlazeBlock for single stride
        let single_block = BlazeBlock::load(
            24,
            28,
            StrideType::Single,
            &variables,
            "backbone1.3.convs.0.weight",
            "backbone1.3.convs.0.bias",
            "backbone1.3.convs.1.weight",
            "backbone1.3.convs.1.bias",
        )
        .unwrap();

        // Call forward method and get the output
        let output = single_block.forward(&output).unwrap(); // (1, 28, 64, 64)
        assert_eq!(output.dims(), &[batch_size, 28, height, width,]);

        // Instantiate the BlazeBlock for double stride
        let double_block = BlazeBlock::load(
            28,
            32,
            StrideType::Double, // stride = 2
            &variables,
            "backbone1.4.convs.0.weight",
            "backbone1.4.convs.0.bias",
            "backbone1.4.convs.1.weight",
            "backbone1.4.convs.1.bias",
        )
        .unwrap();

        // Call forward method and get the output
        let output = double_block.forward(&output).unwrap(); // (1, 32, 32, 32)

        assert_eq!(output.dims(), &[batch_size, 32, height / 2, width / 2,]);
    }
}
