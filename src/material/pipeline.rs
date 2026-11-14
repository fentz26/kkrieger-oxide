//! Original Material11 fixed render states mapped to Miniquad/OpenGL.
use super::State;
use miniquad::*;
impl State {
    /// Depth bits determine OpenGL polygon-offset units for D3D's normalized
    /// constant bias. The renderer must pass the actual depth attachment size.
    pub fn pipeline_params(&self, depth_bits: u32) -> PipelineParams {
        let flags = self.words[0];
        let depth_test = if flags & 0x200 == 0 {
            Comparison::Always
        } else if flags & 0x80 != 0 {
            Comparison::Equal
        } else if flags & 0x40000 != 0 {
            Comparison::Less
        } else {
            Comparison::LessOrEqual
        };
        let cull_face = if flags & 2 != 0 {
            CullFace::Nothing
        } else if flags & 4 != 0 {
            CullFace::Front
        } else {
            CullFace::Back
        };
        let blend = match (flags >> 12) & 15 {
            1 => Some((
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            2 | 6 => Some((BlendFactor::One, BlendFactor::One)),
            3 => Some((
                BlendFactor::Zero,
                BlendFactor::Value(BlendValue::SourceColor),
            )),
            4 => Some((
                BlendFactor::Value(BlendValue::DestinationColor),
                BlendFactor::Value(BlendValue::SourceColor),
            )),
            5 => Some((
                BlendFactor::One,
                BlendFactor::OneMinusValue(BlendValue::SourceColor),
            )),
            7 => Some((
                BlendFactor::OneMinusValue(BlendValue::DestinationColor),
                BlendFactor::OneMinusValue(BlendValue::SourceColor),
            )),
            8 => Some((
                BlendFactor::Value(BlendValue::DestinationAlpha),
                BlendFactor::One,
            )),
            9 => Some((
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::One,
            )),
            _ => None,
        }
        .map(|(src, dst)| {
            BlendState::new(
                if flags & 0xf000 == 0x6000 {
                    Equation::ReverseSubtract
                } else {
                    Equation::Add
                },
                src,
                dst,
            )
        });
        let bias = if flags & 0x10 != 0 {
            -1. / 65536.
        } else if flags & 8 != 0 {
            1. / 65536.
        } else {
            0.
        };
        let mut result = PipelineParams {
            cull_face,
            front_face_order: FrontFaceOrder::Clockwise,
            depth_test,
            depth_write: flags & 0x100 != 0,
            depth_write_offset: if bias != 0. {
                Some((bias, bias * ((2.0f32).powi(depth_bits as i32) - 1.)))
            } else {
                None
            },
            color_blend: blend,
            alpha_blend: blend,
            color_write: if flags & 0x20 != 0 {
                (false, false, false, false)
            } else {
                (true, true, true, true)
            },
            ..Default::default()
        };
        let mut front = StencilFaceState {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op: StencilOp::Keep,
            test_func: CompareFunc::Always,
            test_ref: 0,
            test_mask: u32::MAX,
            write_mask: u32::MAX,
        };
        let mut back = front;
        let zfail = flags & 0x800000 != 0;
        let apply = |face: &mut StencilFaceState, op| {
            if zfail {
                face.depth_fail_op = op;
            } else {
                face.pass_op = op;
            }
        };
        match flags & 0x700000 {
            0x100000 => {
                apply(&mut front, StencilOp::IncrementWrap);
                back = front;
            }
            0x200000 => {
                apply(&mut front, StencilOp::DecrementWrap);
                back = front;
            }
            0x300000 => {
                front.test_func = CompareFunc::Equal;
                back = front;
            }
            0x400000 => {
                front.fail_op = StencilOp::Zero;
                front.depth_fail_op = StencilOp::Zero;
                front.pass_op = StencilOp::Zero;
                back = front;
            }
            0x500000 => {
                result.cull_face = CullFace::Nothing;
                apply(
                    &mut front,
                    if zfail {
                        StencilOp::DecrementWrap
                    } else {
                        StencilOp::IncrementWrap
                    },
                );
                apply(
                    &mut back,
                    if zfail {
                        StencilOp::IncrementWrap
                    } else {
                        StencilOp::DecrementWrap
                    },
                );
            }
            0x600000 => {
                front.test_func = CompareFunc::NotEqual;
                front.pass_op = StencilOp::Zero;
                back = front;
            }
            _ => return result,
        }
        result.stencil_test = Some(StencilState { front, back });
        result
    }
}
