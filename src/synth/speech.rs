//! Original Ronan compressed phonemes, interpolated frames and speech DSP.
mod tables;
pub use tables::{SYLLABLES, Syllable};
#[derive(Clone, Copy, Default)]
pub struct Phoneme {
    pub values: [f32; 19],
}
pub fn phonemes() -> [Phoneme; 69] {
    let multipliers = [
        10, 10, 20, 10, 20, 10, 10, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    ];
    let mut output = [Phoneme::default(); 69];
    let mut value = 0i32;
    for field in 0..19 {
        for (p, phoneme) in output.iter_mut().enumerate() {
            value += i32::from(tables::RAW[field * 69 + p] as i8);
            phoneme.values[field] = value as f32 * multipliers[field] as f32;
        }
    }
    output
}
#[derive(Clone, Copy)]
pub struct Rate {
    pub sample_rate: u32,
    pub angle: f32,
    pub decay: f32,
}
impl Rate {
    pub fn new(sample_rate: u32) -> Result<Self, &'static str> {
        if sample_rate == 0 {
            return Err("invalid Ronan sample rate");
        }
        let angle = std::f32::consts::TAU / sample_rate as f32;
        Ok(Self {
            sample_rate,
            angle,
            decay: -angle * 0.5,
        })
    }
}
#[derive(Clone, Copy, Default)]
pub struct Definition {
    pub a: f32,
    pub b: f32,
    pub c: f32,
}
impl Definition {
    pub fn new(frequency: f32, bandwidth: f32, gain: f32, rate: Rate) -> Self {
        let r = f64::from(rate.decay * bandwidth).exp() as f32;
        let c = -(r * r);
        let b = (r * (f64::from(rate.angle * frequency).cos() as f32)) * 2.;
        let a = gain * ((1. - b) - c);
        Self { a, b, c }
    }
}
#[derive(Clone, Copy, Default)]
pub struct Resonator {
    pub p1: f32,
    pub p2: f32,
}
impl Resonator {
    pub fn tick(&mut self, input: f32, definition: Definition) -> f32 {
        let value = (definition.a * input + definition.b * self.p1) + definition.c * self.p2;
        self.p2 = self.p1;
        self.p1 = value;
        value
    }
}
#[derive(Clone, Copy, Default)]
pub struct Frame {
    pub definitions: [Definition; 7],
    pub amplitudes: [f32; 4],
}
fn interpolate(a: f32, b: f32, x: f32) -> f32 {
    a + x * (b - a)
}
fn decibels(a: f32, b: f32, x: f32) -> f32 {
    2.0f64.powf(f64::from(interpolate(a, b, x) - 70.) / 6.) as f32
}
impl Frame {
    pub fn between(a: &Phoneme, b: &Phoneme, x: f32, pitch: f32, rate: Rate) -> Self {
        let a = a.values;
        let b = b.values;
        let frequencies = |p: [f32; 19]| [p[6], p[0], p[2], p[4], 3200., 4000., 6000.];
        let bandwidths = |p: [f32; 19]| [100., p[1], p[3], p[5], 200., 500., 800.];
        let gains = |p: [f32; 19]| [p[15], p[11], p[12], p[13], p[14], p[16], p[16]];
        let (af, bf) = (frequencies(a), frequencies(b));
        let (ab, bb) = (bandwidths(a), bandwidths(b));
        let (ag, bg) = (gains(a), gains(b));
        Self {
            definitions: std::array::from_fn(|i| {
                Definition::new(
                    interpolate(af[i], bf[i], x) * pitch,
                    interpolate(ab[i], bb[i], x),
                    decibels(ag[i], bg[i], x),
                    rate,
                )
            }),
            amplitudes: std::array::from_fn(|i| decibels(a[7 + i], b[7 + i], x)),
        }
    }
    pub fn values(&self) -> [f32; 25] {
        std::array::from_fn(|i| {
            if i >= 21 {
                self.amplitudes[i - 21]
            } else {
                let d = self.definitions[i / 3];
                [d.a, d.b, d.c][i % 3]
            }
        })
    }
    fn from_values(values: [f32; 25]) -> Self {
        Self {
            definitions: std::array::from_fn(|i| Definition {
                a: values[i * 3],
                b: values[i * 3 + 1],
                c: values[i * 3 + 2],
            }),
            amplitudes: std::array::from_fn(|i| values[21 + i]),
        }
    }
    fn advance(&mut self, delta: Self) {
        for (d, v) in self.definitions.iter_mut().zip(delta.definitions) {
            d.a += v.a;
            d.b += v.b;
            d.c += v.c;
        }
        for (a, v) in self.amplitudes.iter_mut().zip(delta.amplitudes) {
            *a += v;
        }
    }
}
pub struct Dsp {
    pub current: Frame,
    pub target: Frame,
    pub filters: [Resonator; 7],
    pub last_input: f32,
    pub seed: u32,
    pub noise: f32,
    pub highpass: [f32; 2],
    pub equalizer: Resonator,
    pub equalizer_definition: Definition,
}
impl Dsp {
    pub fn new(rate: Rate) -> Self {
        Self {
            current: Frame::default(),
            target: Frame::default(),
            filters: [Resonator::default(); 7],
            last_input: 0.,
            seed: 0,
            noise: 0.,
            highpass: [0.; 2],
            equalizer: Resonator::default(),
            equalizer_definition: Definition::new(12000., 4000., 2., rate),
        }
    }
    fn noise_sample(&mut self) -> f32 {
        self.seed = self.seed.wrapping_mul(196314165).wrapping_add(907633515);
        self.noise = (f32::from_bits((self.seed >> 9) | 0x40000000) - 3.) + 0.75 * self.noise;
        self.noise * 0.25
    }
    pub fn render(&mut self, audio: &mut [[f32; 2]]) {
        if audio.is_empty() {
            return;
        }
        let reciprocal = 1. / audio.len() as f32;
        let current = self.current.values();
        let target = self.target.values();
        let delta = Frame::from_values(std::array::from_fn(|i| {
            (target[i] - current[i]) * reciprocal
        }));
        for frame in audio {
            self.current.advance(delta);
            let amplitudes = self.current.amplitudes;
            let definitions = self.current.definitions;
            let input = frame[0] * amplitudes[0] + self.noise_sample() * amplitudes[1];
            let mut out = self.filters[0].tick(input, definitions[0])
                + self.filters[1].tick(input, definitions[1]);
            let differentiated = (self.noise_sample() * amplitudes[2] + input) - self.last_input;
            self.last_input = input;
            for (filter, &definition) in self.filters[2..].iter_mut().zip(&definitions[2..]) {
                out = filter.tick(differentiated, definition) - out;
            }
            out = differentiated * amplitudes[3] - out;
            out -= self.highpass[0];
            self.highpass[0] += 0.012 * out;
            out -= self.highpass[1];
            self.highpass[1] += 0.012 * out;
            out = self.equalizer.tick(out, self.equalizer_definition) - out;
            *frame = [out; 2];
        }
    }
}

#[derive(Clone, Copy, Default)]
pub struct Sequence {
    pub frame_count: i32,
    pub position_in_syllable: i32,
    pub remaining: i32,
    pub syllable: usize,
    pub duration_factor: i32,
    pub inverse_duration: f32,
    pub text_position: usize,
    pub phonemes: [usize; 2],
    pub wait_on: bool,
    pub wait_off: bool,
}
pub struct Ronan<'a> {
    pub dsp: Dsp,
    pub sequence: Sequence,
    pub pitch: f32,
    pub frame_rate: i32,
    rate: Rate,
    table: [Phoneme; 69],
    lyrics: [Option<&'a [u8]>; 64],
    text: usize,
}
impl Default for Ronan<'_> {
    fn default() -> Self {
        let rate = Rate::new(44100).expect("fixed Ronan default rate");
        Self {
            dsp: Dsp::new(rate),
            sequence: Sequence::default(),
            pitch: 0.,
            frame_rate: 0,
            rate,
            table: phonemes(),
            lyrics: [None; 64],
            text: 0,
        }
    }
}
impl<'a> Ronan<'a> {
    pub fn new(sample_rate: u32) -> Result<Self, &'static str> {
        let mut result = Self::default();
        result.initialize(sample_rate)?;
        Ok(result)
    }
    fn reset_workspace(&mut self) {
        self.dsp.filters = [Resonator::default(); 7];
        self.dsp.last_input = 0.;
        self.dsp.seed = 0;
        self.dsp.noise = 0.;
        self.dsp.highpass = [0.; 2];
        self.dsp.equalizer = Resonator::default();
        self.dsp.current =
            Frame::between(&self.table[18], &self.table[18], 0., self.pitch, self.rate);
        self.dsp.target = self.dsp.current;
        self.sequence = Sequence {
            position_in_syllable: 4,
            phonemes: [18; 2],
            ..Sequence::default()
        };
        self.text = 0;
    }
    pub fn initialize(&mut self, sample_rate: u32) -> Result<(), &'static str> {
        self.rate = Rate::new(sample_rate)?;
        // Init computes the off frames using the preceding pitch, then resets it.
        self.reset_workspace();
        self.frame_rate = 3;
        self.pitch = 1.;
        self.dsp.equalizer_definition = Definition::new(12000., 4000., 2., self.rate);
        Ok(())
    }
    pub fn set_lyrics(&mut self, lyrics: &[Option<&'a [u8]>]) -> Result<(), &'static str> {
        let lyrics = lyrics.get(..64).ok_or("Ronan requires 64 lyric pointers")?;
        self.lyrics.copy_from_slice(lyrics);
        self.text = 0;
        self.sequence.text_position = 0;
        Ok(())
    }
    fn text_byte(&self, position: usize) -> u8 {
        self.lyrics[self.text]
            .unwrap_or(&[])
            .get(position)
            .copied()
            .unwrap_or(0)
    }
    pub fn note_on(&mut self) {
        self.sequence.wait_on = false;
    }
    pub fn note_off(&mut self) {
        self.sequence.wait_off = false;
    }
    pub fn set_control(&mut self, index: u8, value: u8) {
        match index {
            4 if value < 63 => {
                self.reset_workspace();
                self.text = usize::from(value);
            }
            4 => self.frame_rate = i32::from(value) - 63,
            5 => self.pitch = 2.0f64.powf(f64::from((f32::from(value) - 64.) / 128.)) as f32,
            _ => {}
        }
    }
    pub fn tick(&mut self) -> Result<(), &'static str> {
        if self.sequence.wait_on || self.sequence.wait_off {
            return Ok(());
        }
        if self.sequence.frame_count <= 0 {
            self.sequence.frame_count = self.frame_rate;
            if self.sequence.remaining <= 0 {
                self.sequence.position_in_syllable =
                    self.sequence.position_in_syllable.wrapping_add(1);
                let position = self.sequence.position_in_syllable;
                let syllable = SYLLABLES
                    .get(self.sequence.syllable)
                    .ok_or("invalid Ronan syllable")?;
                if position >= 4 || syllable.phonemes.get(position as usize).copied() == Some(-1) {
                    let byte = self.text_byte(self.sequence.text_position);
                    if byte == 0 {
                        self.sequence.duration_factor = 1;
                        self.sequence.frame_count = 1;
                        self.sequence.syllable = SYLLABLES.len() - 1;
                        self.sequence.position_in_syllable = 0;
                        self.sequence.text_position = 0;
                    } else if byte == b'!' || byte == b'_' {
                        self.sequence.frame_count = 0;
                        self.sequence.remaining = 0;
                        self.sequence.wait_on = byte == b'!';
                        self.sequence.wait_off = byte == b'_';
                        self.sequence.text_position += 1;
                        return Ok(());
                    }
                    let byte = self.text_byte(self.sequence.text_position);
                    if byte != 0 && byte != b'!' && byte != b'_' {
                        self.sequence.duration_factor = 0;
                        loop {
                            let byte = self.text_byte(self.sequence.text_position);
                            if !byte.is_ascii_digit() {
                                break;
                            }
                            self.sequence.duration_factor = self
                                .sequence
                                .duration_factor
                                .wrapping_mul(10)
                                .wrapping_add(i32::from(byte - b'0'));
                            self.sequence.text_position += 1;
                        }
                        if self.sequence.duration_factor == 0 {
                            self.sequence.duration_factor = 1;
                        }
                        let mut selected = SYLLABLES.len() - 1;
                        let mut length = 1;
                        for (i, syllable) in SYLLABLES[..SYLLABLES.len() - 1].iter().enumerate() {
                            let n = syllable.name.iter().position(|&v| v == 0).unwrap_or(4);
                            if syllable.name[..n].iter().enumerate().all(|(j, &v)| {
                                v | 0x20 == self.text_byte(self.sequence.text_position + j) | 0x20
                            }) {
                                selected = i;
                                length = n;
                                break;
                            }
                        }
                        self.sequence.syllable = selected;
                        self.sequence.position_in_syllable = 0;
                        self.sequence.text_position += length;
                    }
                }
                let syllable = SYLLABLES
                    .get(self.sequence.syllable)
                    .ok_or("invalid Ronan syllable")?;
                let index = *syllable
                    .phonemes
                    .get(self.sequence.position_in_syllable as usize)
                    .ok_or("invalid Ronan phoneme position")?;
                let phoneme = self
                    .table
                    .get(index as usize)
                    .ok_or("invalid Ronan phoneme")?;
                self.sequence.phonemes = [self.sequence.phonemes[1], index as usize];
                self.sequence.remaining =
                    super::math::integer(phoneme.values[17] * self.sequence.duration_factor as f32);
                if self.sequence.remaining == 0 {
                    self.sequence.remaining = 1;
                }
                self.sequence.inverse_duration =
                    1. / (self.sequence.remaining as f32 * self.frame_rate as f32);
            }
            self.sequence.remaining = self.sequence.remaining.wrapping_sub(1);
        }
        self.sequence.frame_count = self.sequence.frame_count.wrapping_sub(1);
        let position = self
            .sequence
            .remaining
            .wrapping_mul(self.frame_rate)
            .wrapping_add(self.sequence.frame_count);
        let x = position as f32 * self.sequence.inverse_duration;
        let a = self
            .table
            .get(self.sequence.phonemes[0])
            .ok_or("invalid Ronan previous phoneme")?;
        let b = self
            .table
            .get(self.sequence.phonemes[1])
            .ok_or("invalid Ronan current phoneme")?;
        let x = f64::from(x).powf(f64::from(a.values[18] / b.values[18])) as f32;
        self.dsp.target = Frame::between(
            b,
            if (b.values[18] - a.values[18]).abs() > 8. {
                b
            } else {
                a
            },
            x,
            self.pitch,
            self.rate,
        );
        Ok(())
    }
}
impl super::engine::Speech for Ronan<'_> {
    fn reset(&mut self, sample_rate: u32) -> Result<(), &'static str> {
        self.initialize(sample_rate)
    }
    fn note_on(&mut self) -> Result<(), &'static str> {
        Ronan::note_on(self);
        Ok(())
    }
    fn note_off(&mut self) -> Result<(), &'static str> {
        Ronan::note_off(self);
        Ok(())
    }
    fn control(&mut self, index: u8, value: u8) -> Result<(), &'static str> {
        self.set_control(index, value);
        Ok(())
    }
    fn tick(&mut self) -> Result<(), &'static str> {
        Ronan::tick(self)
    }
    fn render(&mut self, channel: &mut [[f32; 2]]) -> Result<(), &'static str> {
        self.dsp.render(channel);
        Ok(())
    }
}
