//! Original ViruZ II channel routing, effects and auxiliary/main accumulation.
use super::{
    boost::Boost,
    compressor::{self, Compressor},
    delay::{self, Delay},
    distortion::{self, Distortion},
    math::integer,
    rate::Rate,
};
#[derive(Clone, Copy)]
pub struct Parameters {
    pub volume: f32,
    pub receive_a: f32,
    pub receive_b: f32,
    pub send_a: f32,
    pub send_b: f32,
    pub auxiliary1: f32,
    pub auxiliary2: f32,
    pub routing: f32,
    pub boost: f32,
    pub distortion: distortion::Parameters,
    pub delay: delay::Parameters,
    pub compressor: compressor::Parameters,
}
impl Parameters {
    pub fn from_values(values: &[f32]) -> Result<Self, &'static str> {
        let p = values.get(..29).ok_or("truncated V2 channel parameters")?;
        Ok(Self {
            volume: p[0],
            receive_a: p[1],
            receive_b: p[2],
            send_a: p[3],
            send_b: p[4],
            auxiliary1: p[5],
            auxiliary2: p[6],
            routing: p[7],
            boost: p[8],
            distortion: distortion::Parameters {
                mode: p[9],
                input_gain: p[10],
                parameter1: p[11],
                parameter2: p[12],
            },
            delay: delay::Parameters {
                amount: p[13],
                feedback: p[14],
                left_length: p[15],
                right_length: p[16],
                rate: p[17],
                depth: p[18],
                phase: p[19],
            },
            compressor: compressor::Parameters {
                mode: p[20],
                stereo: p[21],
                automatic_gain: p[22],
                lookahead: p[23],
                threshold: p[24],
                ratio: p[25],
                attack: p[26],
                release: p[27],
                output_gain: p[28],
            },
        })
    }
}
pub struct Channel {
    pub gain: f32,
    pub auxiliary1_gain: f32,
    pub auxiliary2_gain: f32,
    pub send_a: f32,
    pub send_b: f32,
    pub receive_a: f32,
    pub receive_b: f32,
    pub routing: u32,
    pub boost: Boost,
    pub distortion: Distortion,
    pub delay: Delay,
    pub compressor: Compressor,
}
impl Default for Channel {
    fn default() -> Self {
        Self {
            gain: 0.,
            auxiliary1_gain: 0.,
            auxiliary2_gain: 0.,
            send_a: 0.,
            send_b: 0.,
            receive_a: 0.,
            receive_b: 0.,
            routing: 0,
            boost: Boost::default(),
            distortion: Distortion::default(),
            delay: Delay::new(2048).expect("fixed V2 channel delay size"),
            compressor: Compressor::default(),
        }
    }
}
pub struct Buses<'a> {
    pub mix: &'a mut [[f32; 2]],
    pub auxiliary1: &'a mut [f32],
    pub auxiliary2: &'a mut [f32],
    pub auxiliary_a: &'a mut [[f32; 2]],
    pub auxiliary_b: &'a mut [[f32; 2]],
}
impl Channel {
    pub fn init(&mut self) {
        self.distortion.init();
        self.delay.init();
        self.compressor.init();
        self.boost.init();
    }
    pub fn set(&mut self, p: Parameters, rate: &Rate) -> Result<(), &'static str> {
        self.receive_a = p.receive_a * (1. / 128.);
        self.receive_b = p.receive_b * (1. / 128.);
        self.send_a = (p.send_a * (1. / 128.)) * 0.6;
        self.send_b = (p.send_b * (1. / 128.)) * 0.6;
        self.gain = (p.volume * (1. / 128.)) * 0.6;
        self.auxiliary1_gain = ((p.auxiliary1 * (1. / 128.)) * 0.6) * self.gain;
        self.auxiliary2_gain = ((p.auxiliary2 * (1. / 128.)) * 0.6) * self.gain;
        self.routing = integer(p.routing) as u32;
        self.distortion.set(p.distortion, rate.linear_frequency)?;
        self.delay.set(p.delay, rate);
        self.compressor.set(p.compressor)?;
        self.boost.set(p.boost, rate);
        Ok(())
    }
    // Keep the original source-first mono-send addition order.
    #[allow(clippy::assign_op_pattern)]
    pub fn process(
        &mut self,
        audio: &mut [[f32; 2]],
        buses: &mut Buses<'_>,
        levels: &mut [[f32; 2]],
    ) -> Result<(), &'static str> {
        let n = audio.len();
        if n == 0 {
            return Ok(());
        }
        if buses.mix.len() < n
            || buses.auxiliary1.len() < n
            || buses.auxiliary2.len() < n
            || buses.auxiliary_a.len() < n
            || buses.auxiliary_b.len() < n
            || levels.len() < n
        {
            return Err("V2 channel bus or scratch buffer too short");
        }
        if self.receive_a.to_bits() != 0 {
            accumulate(audio, buses.auxiliary_a, self.receive_a);
        }
        if self.receive_b.to_bits() != 0 {
            accumulate(audio, buses.auxiliary_b, self.receive_b);
        }
        self.compressor.render(audio, levels)?;
        self.boost.render(audio);
        if self.routing == 0 {
            self.distortion
                .render_in_place(audio.as_flattened_mut(), true)?;
            self.delay.render_channel(audio);
        } else {
            self.delay.render_channel(audio);
            self.distortion
                .render_in_place(audio.as_flattened_mut(), true)?;
        }
        for (bus, gain) in [
            (&mut *buses.auxiliary1, self.auxiliary1_gain),
            (&mut *buses.auxiliary2, self.auxiliary2_gain),
        ] {
            if gain.to_bits() != 0 {
                for (output, input) in bus.iter_mut().zip(audio.iter()) {
                    *output = (input[0] + input[1]) * gain + *output;
                }
            }
        }
        if self.send_a.to_bits() != 0 {
            accumulate(buses.auxiliary_a, audio, self.send_a);
        }
        if self.send_b.to_bits() != 0 {
            accumulate(buses.auxiliary_b, audio, self.send_b);
        }
        accumulate(buses.mix, audio, self.gain);
        Ok(())
    }
}
// Preserve the assembly's source-first addition order for non-finite samples.
#[allow(clippy::assign_op_pattern)]
fn accumulate(output: &mut [[f32; 2]], input: &[[f32; 2]], gain: f32) {
    for (out, value) in output.iter_mut().zip(input) {
        out[0] = value[0] * gain + out[0];
        out[1] = value[1] * gain + out[1];
    }
}
