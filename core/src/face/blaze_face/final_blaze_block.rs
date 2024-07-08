// SPDX-FileCopyrightText: © 2023 Mochineko <t.o.e.4315@gmail.com>
//
// SPDX-License-Identifier: MIT
//
// Reference implementation:
// https://github.com/hollance/BlazeFace-PyTorch/blob/master/blazeface.py

use candle_core::{Module, Result, Tensor};
use candle_nn::{Conv2d, Conv2dConfig, VarBuilder};

/// Final BlazeBlock.
pub(crate) struct FinalBlazeBlock {
    conv0: Conv2d,
    conv1: Conv2d,
}

impl FinalBlazeBlock {
    pub(crate) fn load(
        channels: usize,
        variables: &VarBuilder,
        weight_0_name: &str,
        bias_0_name: &str,
        weight_1_name: &str,
        bias_1_name: &str,
    ) -> Result<Self> {
        let weight_0 =
            variables.get_with_hints((channels, 1, 3, 3), weight_0_name, candle_nn::init::ZERO)?;
        let bias_0 = variables.get_with_hints(channels, bias_0_name, candle_nn::init::ZERO)?;
        let weight_1 = variables.get_with_hints(
            (channels, channels, 1, 1),
            weight_1_name,
            candle_nn::init::ZERO,
        )?;
        let bias_1 = variables.get_with_hints(channels, bias_1_name, candle_nn::init::ZERO)?;

        Self::new(channels, weight_0, bias_0, weight_1, bias_1)
    }

    fn new(
        channels: usize,
        weight_0: Tensor,
        bias_0: Tensor,
        weight_1: Tensor,
        bias_1: Tensor,
    ) -> Result<Self> {
        Ok(Self {
            conv0: Conv2d::new(
                weight_0,
                Some(bias_0),
                Conv2dConfig {
                    stride: 2,
                    groups: channels,
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
}

impl Module for FinalBlazeBlock {
    fn forward(&self, input: &Tensor) -> Result<Tensor> {
        let h = input
            .pad_with_zeros(2, 0, 2)? // height padding
            .pad_with_zeros(3, 0, 2)?; // width padding

        let x = self.conv0.forward(&h)?;
        let x = self.conv1.forward(&x)?;
        x.relu()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{safetensors, DType, Device, Tensor};

    #[test]
    fn test_final_blaze_block() {
        // Set up the device
        let device = Device::Cpu;

        // Set up the configuration
        let batch_size = 1;
        let channels = 96;
        let width = 64;
        let height = 64;

        // Set up the convolution parameters
        let weight_0 = Tensor::rand(0., 1., (channels, 1, 3, 3), &device).unwrap();
        let bias_0 = Tensor::rand(0., 1., channels, &device).unwrap();

        let weight_1 = Tensor::rand(0., 1., (channels, channels, 1, 1), &device).unwrap();
        let bias_1 = Tensor::rand(0., 1., channels, &device).unwrap();

        // Instantiate the FinalBlazeBlock
        let block = FinalBlazeBlock::new(channels, weight_0, bias_0, weight_1, bias_1).unwrap();

        // Set up the input Tensor
        let input = Tensor::rand(0., 1., (batch_size, channels, height, width), &device).unwrap(); // (1, 96, 64, 64)

        // Call forward method and get the output
        let output = block.forward(&input).unwrap(); // (1, 96, 32, 32)

        assert_eq!(
            output.dims(),
            &[batch_size, channels, height / 2, width / 2]
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
            safetensors::load("src/blaze_face/data/blazefaceback.safetensors", &device).unwrap();
        let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, &device);

        // Instantiate the FinalBlazeBlock
        let single_block = FinalBlazeBlock::load(
            96,
            &variables,
            "final.convs.0.weight",
            "final.convs.0.bias",
            "final.convs.1.weight",
            "final.convs.1.bias",
        )
        .unwrap();

        // Set up the input Tensor
        let input = Tensor::rand(0., 1., (batch_size, 96, height, width), &device)
            .unwrap()
            .to_dtype(dtype)
            .unwrap(); // (1, 96, 64, 64)

        // Call forward method and get the output
        let output = single_block.forward(&input).unwrap(); // (1, 96, 32, 32)

        assert_eq!(output.dims(), &[batch_size, 96, height / 2, width / 2]);
    }
}
