//! Original ViruZ II modulated delay for channel chorus and mono auxiliary sends.
use super::{math::integer, rate::Rate};
#[derive(Clone, Copy)]
pub struct Parameters {
    pub amount: f32,
    pub feedback: f32,
    pub left_length: f32,
    pub right_length: f32,
    pub rate: f32,
    pub depth: f32,
    pub phase: f32,
}
pub struct Delay {
    pub mask: u32,
    pub write_position: u32,
    pub left_offset: u32,
    pub right_offset: u32,
    pub counter: u32,
    pub frequency: u32,
    pub phase: u32,
    pub depth: u32,
    pub feedback: f32,
    pub dry: f32,
    pub effect: f32,
    pub history: Box<[[f32; 2]]>,
}
impl Delay {
    pub fn new(length: usize) -> Result<Self, &'static str> {
        if !length.is_power_of_two() || length > 32768 {
            return Err("V2 delay length must be a power of two up to 32768");
        }
        Ok(Self {
            mask: (length - 1) as u32,
            write_position: 0,
            left_offset: 0,
            right_offset: 0,
            counter: 0,
            frequency: 0,
            phase: 0,
            depth: 0,
            feedback: 0.,
            dry: 0.,
            effect: 0.,
            history: vec![[0.; 2]; length].into_boxed_slice(),
        })
    }
    pub fn init(&mut self) {
        self.write_position = 0;
        self.counter = 0;
        self.history.fill([0.; 2]);
    }
    pub fn set(&mut self, p: Parameters, rate: &Rate) {
        let amount = (p.amount - 64.) * (1. / 128.);
        self.effect = amount + amount;
        self.dry = 1. - self.effect.abs();
        let feedback = (p.feedback - 64.) * (1. / 128.);
        self.feedback = feedback + feedback;
        let scale = (self.mask as f32 - 1023.) * (1. / 128.);
        self.left_offset = integer(p.left_length * scale) as u32;
        self.right_offset = integer(p.right_length * scale) as u32;
        let frequency = f64::from((p.rate * (1. / 128.) - 1.) * 10.).exp2() as f32;
        self.frequency = integer((frequency * 1973915.5) * rate.linear_frequency) as u32;
        self.depth = integer((p.depth * (1. / 128.)) * 1023.) as u32;
        self.phase =
            (integer(((p.phase - 64.) * (1. / 128.)) * 2147483648.) as u32).wrapping_shl(1);
    }
    fn sample(&mut self, input: [f32; 2], dry: f32) -> [f32; 2] {
        let mut output = [0.; 2];
        // The original processes right first and reads one sample earlier there.
        for side in [1, 0] {
            let phase = if side == 1 {
                self.counter.wrapping_add(self.phase)
            } else {
                self.counter
            };
            let triangle = (phase << 1) ^ if phase & 0x80000000 != 0 { u32::MAX } else { 0 };
            let product = u64::from(triangle) * u64::from(self.depth);
            let offset = (product >> 32) as u32;
            let offset = offset.wrapping_add(if side == 1 {
                self.right_offset
            } else {
                self.left_offset
            });
            let index = self
                .write_position
                .wrapping_sub(offset)
                .wrapping_sub(u32::from(side == 1))
                & self.mask;
            let next = index.wrapping_add(1) & self.mask;
            let fraction = 2. - f32::from_bits(0x3f800000 | ((product as u32) >> 9));
            let first = self.history[index as usize][side];
            let value = first + fraction * (self.history[next as usize][side] - first);
            output[side] = input[side] * dry + value * self.effect;
            self.history[self.write_position as usize][side] = input[side] + value * self.feedback;
        }
        self.counter = self.counter.wrapping_add(self.frequency);
        self.write_position = self.write_position.wrapping_add(1) & self.mask;
        output
    }
    pub fn render_channel(&mut self, audio: &mut [[f32; 2]]) {
        // Test the entire bit pattern: a negative zero follows the active path.
        if self.effect.to_bits() == 0 {
            return;
        }
        for frame in audio {
            let input = frame.map(|v| v + f32::from_bits(0x36800000));
            *frame = self.sample(input, self.dry);
        }
    }
    pub fn render_auxiliary(
        &mut self,
        output: &mut [[f32; 2]],
        input: &[f32],
    ) -> Result<(), &'static str> {
        if input.len() < output.len() {
            return Err("V2 auxiliary delay input too short");
        }
        if self.effect.to_bits() == 0 {
            return Ok(());
        }
        for (frame, &input) in output.iter_mut().zip(input) {
            let input = input + f32::from_bits(0x36800000);
            let value = self.sample([input; 2], 0.);
            // Match the original add-to-main operand order.
            *frame = [value[0] + frame[0], value[1] + frame[1]];
        }
        Ok(())
    }
}
