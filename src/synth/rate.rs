//! Original calcNewSampleRate constants and internal audio-frame length.
#[derive(Clone, Copy)]
pub struct Rate {
    pub oscillator_base: f32,
    pub linear_frequency: f32,
    pub boost_sine: f32,
    pub boost_cosine: f32,
    pub frame_size: u32,
}
impl Rate {
    pub fn new(samples_per_second: u32) -> Result<Self, &'static str> {
        if samples_per_second == 0 || samples_per_second > i32::MAX as u32 {
            return Err("invalid V2 sample rate");
        }
        let reciprocal = (1. / f64::from(samples_per_second)) as f32;
        let oscillator_base = (f32::from_bits(0x4382d013) * 2147483648.) * reciprocal;
        let linear_frequency = 44100. * reciprocal;
        let angle = (reciprocal * std::f32::consts::TAU) * 150.;
        let (sine, cosine) = f64::from(angle).sin_cos();
        let frame = ((samples_per_second as f32 * 128.) / 44100.) + 0.5;
        Ok(Self {
            oscillator_base,
            linear_frequency,
            boost_sine: sine as f32,
            boost_cosine: cosine as f32,
            frame_size: super::math::integer(frame) as u32,
        })
    }
}
