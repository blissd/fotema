// Reference implementation:
// https://github.com/hollance/BlazeFace-PyTorch/blob/master/blazeface.py

use candle_core::{Error, Result, Shape, Tensor};
use candle_nn::{Conv2d, Conv2dConfig, Module, VarBuilder};

use super::{
    blaze_block::{BlazeBlock, StrideType},
    blaze_face::BlazeFaceModel,
    blaze_face_config::DTYPE_IN_BLAZE_FACE,
    final_blaze_block::FinalBlazeBlock,
};

pub(crate) struct BlazeFaceBackModel {
    head: Conv2d,
    backbone: Vec<BlazeBlock>,
    final_block: FinalBlazeBlock,
    classifier_8: Conv2d,
    classifier_16: Conv2d,
    regressor_8: Conv2d,
    regressor_16: Conv2d,
}

impl BlazeFaceBackModel {
    pub(crate) fn load(variables: &VarBuilder) -> Result<Self> {
        let head = Conv2d::new(
            variables.get_with_hints((24, 3, 5, 5), "backbone.0.weight", candle_nn::init::ZERO)?,
            Some(variables.get_with_hints(24, "backbone.0.bias", candle_nn::init::ZERO)?),
            Conv2dConfig {
                stride: 2,
                ..Default::default()
            },
        );

        let backbone = vec![
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.2.convs.0.weight",
                "backbone.2.convs.0.bias",
                "backbone.2.convs.1.weight",
                "backbone.2.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.3.convs.0.weight",
                "backbone.3.convs.0.bias",
                "backbone.3.convs.1.weight",
                "backbone.3.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.4.convs.0.weight",
                "backbone.4.convs.0.bias",
                "backbone.4.convs.1.weight",
                "backbone.4.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.5.convs.0.weight",
                "backbone.5.convs.0.bias",
                "backbone.5.convs.1.weight",
                "backbone.5.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.6.convs.0.weight",
                "backbone.6.convs.0.bias",
                "backbone.6.convs.1.weight",
                "backbone.6.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.7.convs.0.weight",
                "backbone.7.convs.0.bias",
                "backbone.7.convs.1.weight",
                "backbone.7.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.8.convs.0.weight",
                "backbone.8.convs.0.bias",
                "backbone.8.convs.1.weight",
                "backbone.8.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Double,
                variables,
                "backbone.9.convs.0.weight",
                "backbone.9.convs.0.bias",
                "backbone.9.convs.1.weight",
                "backbone.9.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.10.convs.0.weight",
                "backbone.10.convs.0.bias",
                "backbone.10.convs.1.weight",
                "backbone.10.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.11.convs.0.weight",
                "backbone.11.convs.0.bias",
                "backbone.11.convs.1.weight",
                "backbone.11.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.12.convs.0.weight",
                "backbone.12.convs.0.bias",
                "backbone.12.convs.1.weight",
                "backbone.12.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.13.convs.0.weight",
                "backbone.13.convs.0.bias",
                "backbone.13.convs.1.weight",
                "backbone.13.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.14.convs.0.weight",
                "backbone.14.convs.0.bias",
                "backbone.14.convs.1.weight",
                "backbone.14.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.15.convs.0.weight",
                "backbone.15.convs.0.bias",
                "backbone.15.convs.1.weight",
                "backbone.15.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                24,
                StrideType::Single,
                variables,
                "backbone.16.convs.0.weight",
                "backbone.16.convs.0.bias",
                "backbone.16.convs.1.weight",
                "backbone.16.convs.1.bias",
            )?,
            BlazeBlock::load(
                24,
                48,
                StrideType::Double, // stride = 2
                variables,
                "backbone.17.convs.0.weight",
                "backbone.17.convs.0.bias",
                "backbone.17.convs.1.weight",
                "backbone.17.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                48,
                StrideType::Single,
                variables,
                "backbone.18.convs.0.weight",
                "backbone.18.convs.0.bias",
                "backbone.18.convs.1.weight",
                "backbone.18.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                48,
                StrideType::Single,
                variables,
                "backbone.19.convs.0.weight",
                "backbone.19.convs.0.bias",
                "backbone.19.convs.1.weight",
                "backbone.19.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                48,
                StrideType::Single,
                variables,
                "backbone.20.convs.0.weight",
                "backbone.20.convs.0.bias",
                "backbone.20.convs.1.weight",
                "backbone.20.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                48,
                StrideType::Single,
                variables,
                "backbone.21.convs.0.weight",
                "backbone.21.convs.0.bias",
                "backbone.21.convs.1.weight",
                "backbone.21.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                48,
                StrideType::Single,
                variables,
                "backbone.22.convs.0.weight",
                "backbone.22.convs.0.bias",
                "backbone.22.convs.1.weight",
                "backbone.22.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                48,
                StrideType::Single,
                variables,
                "backbone.23.convs.0.weight",
                "backbone.23.convs.0.bias",
                "backbone.23.convs.1.weight",
                "backbone.23.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                48,
                StrideType::Single,
                variables,
                "backbone.24.convs.0.weight",
                "backbone.24.convs.0.bias",
                "backbone.24.convs.1.weight",
                "backbone.24.convs.1.bias",
            )?,
            BlazeBlock::load(
                48,
                96,
                StrideType::Double, // stride = 2
                variables,
                "backbone.25.convs.0.weight",
                "backbone.25.convs.0.bias",
                "backbone.25.convs.1.weight",
                "backbone.25.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone.26.convs.0.weight",
                "backbone.26.convs.0.bias",
                "backbone.26.convs.1.weight",
                "backbone.26.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone.27.convs.0.weight",
                "backbone.27.convs.0.bias",
                "backbone.27.convs.1.weight",
                "backbone.27.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone.28.convs.0.weight",
                "backbone.28.convs.0.bias",
                "backbone.28.convs.1.weight",
                "backbone.28.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone.29.convs.0.weight",
                "backbone.29.convs.0.bias",
                "backbone.29.convs.1.weight",
                "backbone.29.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone.30.convs.0.weight",
                "backbone.30.convs.0.bias",
                "backbone.30.convs.1.weight",
                "backbone.30.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone.31.convs.0.weight",
                "backbone.31.convs.0.bias",
                "backbone.31.convs.1.weight",
                "backbone.31.convs.1.bias",
            )?,
            BlazeBlock::load(
                96,
                96,
                StrideType::Single,
                variables,
                "backbone.32.convs.0.weight",
                "backbone.32.convs.0.bias",
                "backbone.32.convs.1.weight",
                "backbone.32.convs.1.bias",
            )?,
        ];

        let final_block = FinalBlazeBlock::load(
            96,
            variables,
            "final.convs.0.weight",
            "final.convs.0.bias",
            "final.convs.1.weight",
            "final.convs.1.bias",
        )?;

        let classifier_8 = Conv2d::new(
            variables.get_with_hints(
                (2, 96, 1, 1),
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
                (32, 96, 1, 1),
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
            backbone,
            final_block,
            classifier_8,
            classifier_16,
            regressor_8,
            regressor_16,
        })
    }

    fn forward_backbone(&self, input: &Tensor) -> Result<Tensor> {
        let mut x = input.clone();
        for block in &self.backbone {
            x = block.forward(&x)?;
        }
        Ok(x)
    }
}

impl BlazeFaceModel for BlazeFaceBackModel {
    fn forward(
        &self,
        input: &Tensor, // (batch, 3, 256, 256)
    ) -> Result<(Tensor, Tensor)> // coordinates:(batch, 896, 16), score:(batch, 896, 1)
    {
        let batch_size = input.dims()[0];
        if input.dims() != [batch_size, 3, 256, 256] {
            return Result::Err(Error::ShapeMismatchBinaryOp {
                lhs: input.shape().clone(),
                rhs: Shape::from(&[batch_size, 3, 256, 256]),
                op: "forward",
            });
        }
        if input.dtype() != DTYPE_IN_BLAZE_FACE {
            return Result::Err(Error::DTypeMismatchBinaryOp {
                lhs: input.dtype(),
                rhs: DTYPE_IN_BLAZE_FACE,
                op: "forward",
            });
        }
        if !input.device().same_device(self.head.weight().device()) {
            return Result::Err(Error::DeviceMismatchBinaryOp {
                lhs: input.device().location(),
                rhs: self.head.weight().device().location(),
                op: "forward",
            });
        }

        let x = input
            .pad_with_zeros(2, 1, 2)? // height padding
            .pad_with_zeros(3, 1, 2)?; // width padding

        let x = self.head.forward(&x)?; // (batch, 24, 128, 128)
        let x = x.relu()?;
        let x = self.forward_backbone(&x)?; // (batch, 96, 16, 16)

        let h = self.final_block.forward(&x)?; // (batch, 96, 8, 8)

        let c1 = self.classifier_8.forward(&x)?; // (batch, 2, 16, 16)
        let c1 = c1.permute((0, 2, 3, 1))?; // (batch, 16, 16, 2)
        let c1 = c1.reshape((batch_size, 512, 1))?; // (batch, 512, 1)

        let c2 = self.classifier_16.forward(&h)?; // (batch, 6, 8, 8)
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

        let safetensors =
            safetensors::load("src/blaze_face/data/blazefaceback.safetensors", &device).unwrap();
        let variables = candle_nn::VarBuilder::from_tensors(safetensors, dtype, &device);

        // Load the model
        let model = BlazeFaceBackModel::load(&variables).unwrap();

        // Set up the input Tensor
        let input = Tensor::zeros((batch_size, 3, 256, 256), dtype, &device).unwrap();

        // Call forward method and get the output
        let output = model.forward(&input).unwrap();

        assert_eq!(output.0.dims(), &[batch_size, 896, 16]);
        assert_eq!(output.1.dims(), &[batch_size, 896, 1]);
    }
}
