// SPDX-FileCopyrightText: © 2023 Mochineko <t.o.e.4315@gmail.com>
//
// SPDX-License-Identifier: MIT
//
// Reference implementation:
// https://github.com/hollance/BlazeFace-PyTorch/blob/master/blazeface.py

use candle_core::{Module, Result, Tensor};
use candle_nn::{Conv2d, Conv2dConfig, VarBuilder};

use super::{
    blaze_block::{BlazeBlock, StrideType},
    blaze_face::BlazeFaceModel,
};

pub(crate) struct BlazeFaceFrontModel {
    head: Conv2d,
    backbone_1: Vec<BlazeBlock>,
    backbone_2: Vec<BlazeBlock>,
    classifier_8: Conv2d,
    classifier_16: Conv2d,
    regressor_8: Conv2d,
    regressor_16: Conv2d,
}

impl BlazeFaceFrontModel {
    pub(crate) fn load(variables: &VarBuilder) -> Result<Self> {
        let head = Conv2d::new(
            variables.get_with_hints((24, 3, 5, 5), "backbone1.0.weight", candle_nn::init::ZERO)?,
            Some(variables.get_with_hints(24, "backbone1.0.bias", candle_nn::init::ZERO)?),
            Conv2dConfig {
                stride: 2,
                ..Default::default()
            },
        );

        let backbone_1 = vec![
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone1.2.convs.0.weight",
                "backbone1.2.convs.0.bias",
                "backbone1.2.convs.1.weight",
                "backbone1.2.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                28,
                StrideType::Single,
                variables,
                "backbone1.3.convs.0.weight",
                "backbone1.3.convs.0.bias",
                "backbone1.3.convs.1.weight",
                "backbone1.3.convs.1.bias",
            )?,
            BlazeBlock::load(
                28,
                32,
                StrideType::Double, // stride = 2
                variables,
                "backbone1.4.convs.0.weight",
                "backbone1.4.convs.0.bias",
                "backbone1.4.convs.1.weight",
                "backbone1.4.convs.1.bias",
            )?,
            BlazeBlock::load(
                32,
                36,
                StrideType::Single,
                variables,
                "backbone1.5.convs.0.weight",
                "backbone1.5.convs.0.bias",
                "backbone1.5.convs.1.weight",
                "backbone1.5.convs.1.bias",
            )?,
            BlazeBlock::load(
                36,
                42,
                StrideType::Single,
                variables,
                "backbone1.6.convs.0.weight",
                "backbone1.6.convs.0.bias",
                "backbone1.6.convs.1.weight",
                "backbone1.6.convs.1.bias",
            )?,
            BlazeBlock::load(
                42,
                48,
                StrideType::Double, // stride = 2
                variables,
                "backbone1.7.convs.0.weight",
                "backbone1.7.convs.0.bias",
                "backbone1.7.convs.1.weight",
                "backbone1.7.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                56,
                StrideType::Single,
                variables,
                "backbone1.8.convs.0.weight",
                "backbone1.8.convs.0.bias",
                "backbone1.8.convs.1.weight",
                "backbone1.8.convs.1.bias",
            )?,
            BlazeBlock::load(
                56,
                64,
                StrideType::Single,
                variables,
                "backbone1.9.convs.0.weight",
                "backbone1.9.convs.0.bias",
                "backbone1.9.convs.1.weight",
                "backbone1.9.convs.1.bias",
            )?,
            BlazeBlock::load(
                64,
                72,
                StrideType::Single,
                variables,
                "backbone1.10.convs.0.weight",
                "backbone1.10.convs.0.bias",
                "backbone1.10.convs.1.weight",
                "backbone1.10.convs.1.bias",
            )?,
            BlazeBlock::load(
                72,
                80,
                StrideType::Single,
                variables,
                "backbone1.11.convs.0.weight",
                "backbone1.11.convs.0.bias",
                "backbone1.11.convs.1.weight",
                "backbone1.11.convs.1.bias",
            )?,
            BlazeBlock::load(
                80,
                88,
                StrideType::Single,
                variables,
                "backbone1.12.convs.0.weight",
                "backbone1.12.convs.0.bias",
                "backbone1.12.convs.1.weight",
                "backbone1.12.convs.1.bias",
            )?,
        ];

        let backbone_2 = vec![
            BlazeBlock::load(
                88,
                96,
                StrideType::Double, // stride = 2
                variables,
                "backbone2.0.convs.0.weight",
                "backbone2.0.convs.0.bias",
                "backbone2.0.convs.1.weight",
                "backbone2.0.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone2.1.convs.0.weight",
                "backbone2.1.convs.0.bias",
                "backbone2.1.convs.1.weight",
                "backbone2.1.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone2.2.convs.0.weight",
                "backbone2.2.convs.0.bias",
                "backbone2.2.convs.1.weight",
                "backbone2.2.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone2.3.convs.0.weight",
                "backbone2.3.convs.0.bias",
                "backbone2.3.convs.1.weight",
                "backbone2.3.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone2.4.convs.0.weight",
                "backbone2.4.convs.0.bias",
                "backbone2.4.convs.1.weight",
                "backbone2.4.convs.1.bias",
            )?,
        ];

        let classifier_8 = Conv2d::new(
            variables.get_with_hints(
                (2, 88, 1, 1),
                "classifier_8.weight",
                candle_nn::init::ZERO,
            )?,
            Some(variables.get_with_hints((2,), "classifier_8.bias", candle_nn::init::ZERO)?),
            Conv2dConfig {
                ..Default::default()
            },
        );

        let classifier_16 = Conv2d::new(
            variables.get_with_hints(
                (6, 96, 1, 1),
                "classifier_16.weight",
                candle_nn::init::ZERO,
            )?,
            Some(variables.get_with_hints((6,), "classifier_16.bias", candle_nn::init::ZERO)?),
            Conv2dConfig {
                ..Default::default()
            },
        );

        let regressor_8 = Conv2d::new(
            variables.get_with_hints(
                (32, 88, 1, 1),
                "regressor_8.weight",
                candle_nn::init::ZERO,
            )?,
            Some(variables.get_with_hints((32,), "regressor_8.bias", candle_nn::init::ZERO)?),
            Conv2dConfig {
                ..Default::default()
            },
        );

        let regressor_16 = Conv2d::new(
            variables.get_with_hints(
                (96, 96, 1, 1),
                "regressor_16.weight",
                candle_nn::init::ZERO,
            )?,
            Some(variables.get_with_hints((96,), "regressor_16.bias", candle_nn::init::ZERO)?),
            Conv2dConfig {
                ..Default::default()
            },
        );

        Ok(Self {
            head,
            backbone_1,
            backbone_2,
            classifier_8,
            classifier_16,
            regressor_8,
            regressor_16,
        })
    }

    fn forward_backbone_1(&self, input: &Tensor) -> Result<Tensor> {
        let mut x = input.clone();
        for block in &self.backbone_1 {
            x = block.forward(&x)?;
        }
        Ok(x)
    }

    fn forward_backbone_2(&self, input: &Tensor) -> Result<Tensor> {
        let mut x = input.clone();
        for block in &self.backbone_2 {
            x = block.forward(&x)?;
        }
        Ok(x)
    }
}

impl BlazeFaceModel for BlazeFaceFrontModel {
    fn forward(
        &self,
        input: &Tensor, // (batch, 3, 128, 128)
    ) -> Result<(Tensor, Tensor)> // coodinates:(batch, 896, 16), score:(batch, 896, 1),
    {
        let batch_size = input.dims()[0];

        let x = input
            .pad_with_zeros(2, 1, 2)? // height padding
            .pad_with_zeros(3, 1, 2)?; // width padding

        let x = self.head.forward(&x)?; // (batch, 24, 64, 64)
        let x = x.relu()?;
        let x = self.forward_backbone_1(&x)?; // (batch, 88, 16, 16)

        let h = self.forward_backbone_2(&x)?; // (batch, 96, 8, 8)

        let c1 = self.classifier_8.forward(&x)?; // (batch, 2, 16, 16)
        let c1 = c1.permute((0, 2, 3, 1))?; // (batch, 16, 16, 2)
        let c1 = c1.reshape((batch_size, 512, 1))?; // (batch, 512, 1)

        let c2 = self.classifier_16.forward(&h)?; // # (batch, 6, 8, 8)
        let c2 = c2.permute((0, 2, 3, 1))?; // (batch, 8, 8, 6)
        let c2 = c2.reshape((batch_size, 384, 1))?; // (batch, 384, 1)

        let c = Tensor::cat(&[c1, c2], 1)?; // (batch, 896, 1)

        let r1 = self.regressor_8.forward(&x)?; // (batch, 32, 16, 16)
        let r1 = r1.permute((0, 2, 3, 1))?; // (batch, 16, 16, 32)
        let r1 = r1.reshape((batch_size, 512, 16))?; // (batch, 512, 16)

        let r2 = self.regressor_16.forward(&h)?; // (batch, 96, 8, 8)
        let r2 = r2.permute((0, 2, 3, 1))?; // (batch, 8, 8, 96)
        let r2 = r2.reshape((batch_size, 384, 16))?; // (batch, 384, 16)

        let r = Tensor::cat(&[r1, r2], 1)?; // (batch, 896, 16)

        Ok((r, c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::{safetensors, DType, Device, Tensor};

    #[test]
    fn test_forward() {
        // Set up the device and dtype
        let device = Device::Cpu;
        let dtype = DType::F16;
        let batch_size = 1;

        // Load the variables
        let safetensors =
            safetensors::load("src/face/blaze_face/data/blazeface.pth", &device).unwrap();
        let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, &device);

        // Load the model
        let model = BlazeFaceFrontModel::load(&variables).unwrap();

        // Set up the input Tensor
        let input = Tensor::zeros((batch_size, 3, 128, 128), dtype, &device)
            .unwrap()
            .to_dtype(dtype)
            .unwrap();

        // Call forward method and get the output
        let output = model.forward(&input).unwrap();

        assert_eq!(output.0.dims(), &[batch_size, 896, 16]);
        assert_eq!(output.1.dims(), &[batch_size, 896, 1]);
    }
}
