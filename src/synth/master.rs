//! Original ViruZ II global effects, output filters and sum compressor.
use super::{
    compressor::{self, Compressor},
    delay::{self, Delay},
    math::Float,
    rate::Rate,
    reverb::{self, Reverb},
};
pub struct Master {
    pub parameters: [f32; 22],
    pub reverb: Reverb,
    pub delay: Delay,
    pub compressor: Compressor,
    pub low_frequency: f32,
    pub low_state: [f32; 2],
    pub high_frequency: f32,
    pub high_state: [f32; 2],
}
impl Default for Master {
    fn default() -> Self {
        Self {
            parameters: [0.; 22],
            reverb: Reverb::default(),
            delay: Delay::new(32768).expect("fixed V2 master delay size"),
            compressor: Compressor::default(),
            low_frequency: 0.,
            low_state: [0.; 2],
            high_frequency: 0.,
            high_state: [0.; 2],
        }
    }
}
impl Master {
    pub fn init(&mut self) {
        self.reverb.init();
        self.delay.init();
        self.compressor.init();
    }
    pub fn set(&mut self, globals: &[u8], rate: &Rate) -> Result<(), &'static str> {
        let globals = globals.get(..22).ok_or("truncated V2 global parameters")?;
        let p = std::array::from_fn::<_, 22, _>(|i| f32::from(globals[i]));
        self.parameters = p;
        self.reverb.set(
            reverb::Parameters {
                time: p[0],
                high_cut: p[1],
                low_cut: p[2],
                volume: p[3],
            },
            rate,
        );
        self.delay.set(
            delay::Parameters {
                amount: p[4],
                feedback: p[5],
                left_length: p[6],
                right_length: p[7],
                rate: p[8],
                depth: p[9],
                phase: p[10],
            },
            rate,
        );
        self.compressor.set(compressor::Parameters {
            mode: p[13],
            stereo: p[14],
            automatic_gain: p[15],
            lookahead: p[16],
            threshold: p[17],
            ratio: p[18],
            attack: p[19],
            release: p[20],
            output_gain: p[21],
        })?;
        let low = (p[11] + 1.) * (1. / 128.);
        self.low_frequency = low * low;
        let high = (p[12] + 1.) * (1. / 128.);
        self.high_frequency = high * high;
        Ok(())
    }
    fn filter(&mut self, mix: &mut [[f32; 2]]) {
        let mut low = self.low_state.map(Float::from);
        let mut high = self.high_state.map(Float::from);
        let lf = Float::from(self.low_frequency);
        let hf = Float::from(self.high_frequency);
        let bypass = self.high_frequency.to_bits() == 1f32.to_bits();
        let mut underflow = false;
        for frame in mix {
            for side in 0..2 {
                let mut value = Float::from(frame[side]) - low[side];
                low[side] = low[side] + value * lf;
                if !bypass {
                    high[side] = high[side] + (value - high[side]) * hf;
                    value = high[side];
                }
                frame[side] = value.store(&mut underflow);
            }
        }
        self.low_state = low.map(|v| v.store(&mut underflow));
        self.high_state = high.map(|v| v.store(&mut underflow));
    }
    pub fn process(
        &mut self,
        mix: &mut [[f32; 2]],
        auxiliary1: &[f32],
        auxiliary2: &[f32],
        levels: &mut [[f32; 2]],
    ) -> Result<(), &'static str> {
        let n = mix.len();
        if n == 0 {
            return Ok(());
        }
        if auxiliary1.len() < n || auxiliary2.len() < n || levels.len() < n {
            return Err("V2 master auxiliary or scratch buffer too short");
        }
        self.reverb.process(mix, auxiliary1)?;
        self.delay.render_auxiliary(mix, auxiliary2)?;
        self.filter(mix);
        self.compressor.render(mix, levels)
    }
}
